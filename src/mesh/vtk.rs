use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::model::{CellType, Cells, VertexNumbers};
use vtkio::*;

pub fn load_vtk(path: &PathBuf) -> Vtk {
    let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
    if let Ok(vtk_file) = Vtk::import(&vtk_path) {
        vtk_file
    } else {
        panic!("Failed to load VTK file: {}", vtk_path.display());
    }
}
pub fn process_vtk_mesh(vtk: &Vtk) -> Option<Mesh> {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    match &vtk.data {
        // 1. UnstructuredGrid
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            let mut vertices_all_pieces: Vec<Vec<[f32; 3]>> = Vec::new(); //暂时不用，视情况看以后有了多pieces的数据再看
            let mut vertices: Vec<[f32; 3]> = Vec::new();
            let mut indices_all_pieces: Vec<Vec<u32>> = Vec::new(); //暂时不用，视情况看以后有了多pieces的数据再看
            let mut indices: Vec<u32> = Vec::new();
            for piece in pieces {
                match piece {
                    // 1.1 process vertices data
                    // Piece::Inline
                    model::Piece::Inline(grid_piece) => {
                        let points = grid_piece
                            .points
                            .cast_into::<f32>()
                            .expect("IOBuffer converted failed.");

                        let vertices_per_piece: Vec<[f32; 3]> =
                            points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();

                        vertices_all_pieces.push(vertices_per_piece.clone());
                        vertices.append(&mut vertices_per_piece.clone());

                        // 1.2 process vertices indices
                        let mut indices_per_piece = Vec::new();
                        process_vtk_cells(
                            vertices_per_piece,
                            &mut indices_per_piece,
                            &grid_piece.cells,
                        );

                        indices.append(&mut indices_per_piece);
                        indices_all_pieces.push(indices_per_piece);
                    }

                    // Piece::{Source, Loaded}
                    _ => {
                        println!("Source or Loaded");
                    }
                }
            }

            // 1.2 create mesh
            println!("Finished processing mesh. mesh vertices: {:?}", vertices);
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::from(vertices.clone()),
            );

            println!("indices:{:?}", indices);
            mesh.insert_indices(Indices::U32(indices.clone()));

            mesh.compute_normals();

            Some(mesh)
        }
        // enum ImageData, StructuredGrid, RectilinearGrid, PolyData, Field
        _ => None,
    }
}

fn process_vtk_cells(
    // TODO: Unused variable
    _vertices_per_piece: Vec<[f32; 3]>,
    indices_per_piece: &mut Vec<u32>,
    cells: &Cells,
) {
    match &cells.cell_verts {
        VertexNumbers::Legacy {
            num_cells: _,
            vertices,
        } => {
            // read data from vertices
            // e.g. vertices = [4, 0, 1, 2, 3]:
            // 4 : number of vertices
            // [0,1,2,3] : index
            let mut current_index = 0;
            for cell_type in &cells.types {
                let n_vertices = vertices[current_index] as usize;
                let vertex_indices =
                    vertices[current_index + 1..current_index + 1 + n_vertices].to_vec();
                match cell_type {
                    CellType::Tetra => {
                        indices_per_piece.extend_from_slice(&[
                            vertex_indices[0],
                            vertex_indices[2],
                            vertex_indices[1], // face 1
                            vertex_indices[0],
                            vertex_indices[3],
                            vertex_indices[2], // face 2
                            vertex_indices[0],
                            vertex_indices[1],
                            vertex_indices[3], // face 3
                            vertex_indices[1],
                            vertex_indices[2],
                            vertex_indices[3], // face 4
                        ]);
                    }
                    _ => println!("TODO: Unsupported cell type: {:?}", cell_type),
                }
                current_index += 1 + n_vertices;
            }
            // println!("{:?}", num_cells);
        }
        VertexNumbers::XML {
            connectivity,
            offsets,
        } => {
            println!("{:?}, {:?}", connectivity, offsets);
        }
    }
}
