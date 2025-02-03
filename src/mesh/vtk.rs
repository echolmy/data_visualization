use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::*;

/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field

pub trait VtkMeshExtractor {
    // basic geometry process
    fn extract_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]>;
    fn extract_indices(&self, cells: model::Cells) -> Vec<u32>;
}
pub struct CommonMeshExtractor;
impl VtkMeshExtractor for CommonMeshExtractor {
    fn extract_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]> {
        // process point position
        let points = points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        // construct position of each vertices
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
    }
    fn extract_indices(&self, cells: model::Cells) -> Vec<u32> {
        triangulate_cells(cells)
    }
}

// general triangulate cells
pub fn triangulate_cells(cells: model::Cells) -> Vec<u32> {
    // allocate memory according to triangle intially, if small, it will re-allocate
    let mut indices = Vec::<u32>::with_capacity(cells.num_cells() * 3);
    let cell_data = cells.cell_verts.into_legacy();

    // create iterator
    // use peekable to check edge condition
    let mut data_iter = cell_data.1.iter().copied().peekable();

    for cell_type in &cells.types {
        if data_iter.peek().is_none() {
            panic!("Cell type list longer than available data");
        }
        // load the number of each cell (first number of each row of cell)
        let num_vertices = data_iter.next().expect("Missing vertex count") as usize;
        // load indices of vertices of this cell
        let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

        // process data according to topology
        match cell_type {
            // triangle
            model::CellType::Triangle => {
                if num_vertices != 3 {
                    panic!(
                        "Invalid triangle vertex count: {} (expected 3)",
                        num_vertices
                    );
                }
                // push indices of this cell to indices list
                indices.extend(vertices);
            }

            model::CellType::Polygon => {
                // 多边形三角剖分（改进版ear clipping算法）
                // triangulate_polygon(&vertices, &mut indices);
                todo!()
            }

            _ => {
                println!("Unsupported cell type: {:?}", cell_type);
                todo!()
            }
        }
    }
    // check whether data all consumed
    if data_iter.next().is_some() {
        panic!(
            "{} bytes of extra data remaining after processing",
            data_iter.count() + 1
        );
    }
    indices
}

pub fn process_unstructured_grid(
    mut pieces: Vec<model::Piece<model::UnstructuredGridPiece>>,
    extractor: &impl VtkMeshExtractor,
) -> Option<Mesh> {
    if let model::Piece::Inline(piece) = pieces.remove(0) {
        Some(create_mesh_legacy(extractor, &piece.points, piece.cells))
    } else {
        None
    }
}
pub fn process_polydata(pieces: &Vec<model::Piece<model::PolyDataPiece>>) {
    for piece in pieces {
        if let model::Piece::Inline(ptr) = &pieces[0] {
            todo!()
        }
    }
}

//************************************* Main Process Logic**************************************//
fn _load_vtk(path: &PathBuf) -> Vtk {
    let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
    if let Ok(vtk_file) = Vtk::import(&vtk_path) {
        vtk_file
    } else {
        panic!("Failed to load VTK file: {}", vtk_path.display());
    }
}

pub fn process_vtk_file_legacy(path: &PathBuf) -> Option<Mesh> {
    let vtk = _load_vtk(path);
    let extractor = CommonMeshExtractor;

    match vtk.data {
        // Unstructured Grid
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            process_unstructured_grid(pieces, &extractor)
        }
        model::DataSet::PolyData { meta: _, pieces } => {
            println!("{:?}", pieces);
            todo!("----")
        }
        _ => {
            todo!("Unsupported dataSet. Please use unstructured grid or PolyData")
        }
    }
}

pub fn create_mesh_legacy(
    extractor: &impl VtkMeshExtractor,
    points: &IOBuffer,
    cells: model::Cells,
) -> Mesh {
    // initialize a mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // process vertices position attributes
    let vertices = extractor.extract_vertices(points);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(vertices),
    );

    // process vertices indices attributes
    let indices = extractor.extract_indices(cells);
    mesh.insert_indices(Indices::U32(indices));

    // compute normals
    mesh.compute_normals();

    mesh
}

//**************************************************************************//
