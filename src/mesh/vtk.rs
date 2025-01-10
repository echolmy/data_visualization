use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::model::Piece;
use vtkio::*;
/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field
pub trait VtkDataSetProcessor<T> {
    fn process_legacy(&self, pieces: Vec<model::Piece<T>>) -> Option<Mesh>;

    #[allow(unused)]
    fn process_xml(&self, dataset: &model::DataSet) -> Option<Mesh>;
}

// Processor for [Unstructured Grid]
pub struct UnstructuredGridProcessor {
    supported_cell_types: Vec<model::CellType>,
}

impl UnstructuredGridProcessor {
    pub fn new(supported_cell_types: Vec<model::CellType>) -> Self {
        Self {
            supported_cell_types,
        }
    }
    fn _process_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]> {
        // process point position
        let points = points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        // construct position of each vertices
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
    }

    fn _process_cell_indices(&self, cells: model::Cells) -> Vec<u32> {
        match &cells.cell_verts {
            model::VertexNumbers::Legacy {
                num_cells: _,
                vertices: _,
            } => {
                // check every cell type
                for _type in &cells.types {
                    if *_type != model::CellType::Triangle {
                        panic!("cell type contains non-triangle");
                    }
                }
                let mut indices: Vec<u32> = Vec::with_capacity(cells.num_cells() * 3);
                let cell_data = cells.cell_verts.into_legacy();
                if cell_data.1.len() % 4 != 0 {
                    panic!("number of [CELLS] are not multiply of 4.")
                }

                for chunk in cell_data.1.chunks_exact(4) {
                    indices.push(chunk[1]);
                    indices.push(chunk[2]);
                    indices.push(chunk[3]);
                }
                indices
            }
            model::VertexNumbers::XML {
                connectivity: _,
                offsets: _,
            } => {
                todo!()
            }
        }
    }
}

impl VtkDataSetProcessor<model::UnstructuredGridPiece> for UnstructuredGridProcessor {
    fn process_legacy(
        &self,
        mut pieces: Vec<model::Piece<model::UnstructuredGridPiece>>,
    ) -> Option<Mesh> {
        for supported_celltype in &self.supported_cell_types {
            if *supported_celltype != model::CellType::Triangle {
                panic!("unsupported cell types: non triangles")
            }
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );

        // legacy should be only one piece::inline
        if let Some(Piece::Inline(_)) = pieces.first() {
            if let Piece::Inline(piece_ptr) = pieces.remove(0) {
                // 1. process geometry data
                let vertices = self._process_vertices(&piece_ptr.points);
                let indices = self._process_cell_indices(piece_ptr.cells);

                // 2. insert data to mesh
                // 2.1. insert points positions
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    VertexAttributeValues::from(vertices),
                );
                // 2.2. insert points indices
                mesh.insert_indices(Indices::U32(indices));

                // 2.3 compute normals
                mesh.compute_normals();
                Some(mesh)
            } else {
                panic!("[Legacy format] Pieces array is empty. Please check.")
            }
        } else {
            panic!("First piece must be an Inline piece")
        }
    }
    //TODO
    fn process_xml(&self, _dataset: &model::DataSet) -> Option<Mesh> {
        todo!()
    }
}

// pub fn process_vtk_mesh_legacy(vtk: Vtk) -> Option<Mesh> {
//     let mut mesh = Mesh::new(
//         PrimitiveTopology::TriangleList,
//         RenderAssetUsages::default(),
//     );

//     match vtk.data {
//         // 1. UnstructuredGrid
//         model::DataSet::UnstructuredGrid { meta: _, pieces } => {
//             _process_legacy_unstructured_grid(&mut mesh, pieces);
//             Some(mesh)
//         }
//         _ => {
//             println!("Unsupported Other DataSet. Please select UnstructuredGrid");
//             None
//         }
//     }
// }

// fn _process_legacy_unstructured_grid(
//     mesh: &mut Mesh,
//     mut pieces: Vec<Piece<model::UnstructuredGridPiece>>,
// ) {
//     let mut vertices: Vec<[f32; 3]> = Vec::new();
//     let indices: Vec<u32>;
//     // legacy should be only one piece::inline

//     // 1. process geometry data
//     if let Some(Piece::Inline(_)) = pieces.first() {
//         if let Piece::Inline(piece_ptr) = pieces.remove(0) {
//             // 1.1 process point position
//             let points = piece_ptr
//                 .points
//                 .cast_into::<f32>()
//                 .expect("IOBuffer converted failed.");
//             vertices.reserve(piece_ptr.num_points());
//             vertices = points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();

//             // 1.2. process point indices to construct triangle
//             indices = _process_legacy_unstructuredgrid_cells_triangle(piece_ptr);
//         } else {
//             panic!("[Legacy format] Pieces array is empty. Please check.")
//         }
//     } else {
//         panic!("First piece must be an Inline piece")
//     }

//     // 2. insert data to mesh
//     // 2.1. insert points positions
//     mesh.insert_attribute(
//         Mesh::ATTRIBUTE_POSITION,
//         VertexAttributeValues::from(vertices),
//     );
//     // 2.2. insert points indices
//     mesh.insert_indices(Indices::U32(indices));

//     // 2.3 compute normals
//     mesh.compute_normals();
// }

// fn _process_legacy_unstructuredgrid_cells_triangle(
//     piece_ptr: Box<model::UnstructuredGridPiece>,
// ) -> Vec<u32> {
//     // check every cell type
//     for _type in &piece_ptr.cells.types {
//         if *_type != model::CellType::Triangle {
//             panic!("cell type contains non-triangle");
//         }
//     }
//     let mut indices: Vec<u32> = Vec::with_capacity(piece_ptr.cells.num_cells() * 3);

//     // clone operation cannot avoid because of `into_legacy()` operation
//     let cell_data = piece_ptr.cells.cell_verts.into_legacy();

//     if cell_data.1.len() % 4 != 0 {
//         panic!("number of [CELLS] are not multiply of 4.")
//     }

//     for chunk in cell_data.1.chunks_exact(4) {
//         indices.push(chunk[1]);
//         indices.push(chunk[2]);
//         indices.push(chunk[3]);
//     }

//     indices
// }

pub fn process_vtk_file_legacy(path: &PathBuf) -> Option<Mesh> {
    let vtk = _load_vtk(path);
    match vtk.data {
        // Unstructured Grid
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            let processor = UnstructuredGridProcessor::new(vec![model::CellType::Triangle]);
            processor.process_legacy(pieces)
        }
        _ => {
            todo!("Unsupported dataSet. Please use unstructured grid")
        }
    }
}

//TODO
#[allow(unused)]
pub fn process_vtk_file_xml(path: &PathBuf) -> Option<Mesh> {
    let mut vtk = _load_vtk(path);
    todo!()
}

fn _load_vtk(path: &PathBuf) -> Vtk {
    let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
    if let Ok(vtk_file) = Vtk::import(&vtk_path) {
        vtk_file
    } else {
        panic!("Failed to load VTK file: {}", vtk_path.display());
    }
}
