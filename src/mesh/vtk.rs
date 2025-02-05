use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::*;

/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field
pub struct GeometryData {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    // normals: Option<Vec<[f32; 3]>>,
}
#[derive(Debug)]
pub enum VtkError {
    LoadError(String),
    InvalidFormat(&'static str),
    UnsupportedDataType,
    MissingData(&'static str),
    // ...other errors
}
pub trait VtkMeshExtractor {
    // associated type
    type PieceType;

    // basic geometry process
    fn extract_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]> {
        // process point position
        let points = points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        // construct position of each vertices
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
    }

    fn extract_indices(&self, cells: model::Cells) -> Vec<u32>;

    // fn extract_geometry(&self, dataset: model::DataSet) -> Result<GeometryData, VtkError> {
    //     match dataset {
    //         model::DataSet::UnstructuredGrid { meta: _, pieces } => self.process_legacy(pieces),
    //         model::DataSet::PolyData { meta, pieces } => {
    //             // self.process_legacy(pieces)
    //             todo!()
    //         }
    //         _ => Err(VtkError::UnsupportedDataType),
    //     }
    // }

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError>;
}
// pub struct MeshExtractor;
pub struct UnstructuredGridExtractor;
pub struct PolyDataExtractor;
impl VtkMeshExtractor for UnstructuredGridExtractor {
    type PieceType = Vec<model::Piece<model::UnstructuredGridPiece>>;
    fn extract_indices(&self, cells: model::Cells) -> Vec<u32> {
        self.triangulate_cells(cells)
    }

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .into_iter()
            .next()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(piece.cells);

        // use bevy interface to compute normals
        // let normals = compute_normals(&vertices, &indices);

        Ok(GeometryData { vertices, indices })
    }
}
impl UnstructuredGridExtractor {
    // general triangulate cells
    fn triangulate_cells(&self, cells: model::Cells) -> Vec<u32> {
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
}

// impl VtkMeshExtractor for PolyDataExtractor {
//     fn extract_indices(&self, cells: model::Cells) -> Vec<u32> {
//         todo!()
//     }

//     fn extract_geometry(&self, dataset: model::DataSet) -> Result<GeometryData, VtkError> {
//         todo!()
//     }
// }

// impl PolyDataExtractor {
//     pub fn process_polydata(
//         &self,
//         pieces: Vec<model::Piece<model::PolyDataPiece>>,
//     ) -> Result<GeometryData, VtkError> {
//         let piece = pieces
//             .into_iter()
//             .next()
//             .ok_or(VtkError::MissingData("No pieces found".into()))?;
//         let model::Piece::Inline(piece) = piece else {
//             return Err(VtkError::InvalidFormat("Expected inline data".into()));
//         };
//         let vertices = self.extract_vertices(&piece.points);

//         let mut indices = Vec::<u32>::new();
//         // let verts = piece.verts.as_ref().ok_or()
//         let polys = piece.polys.as_ref().ok_or(VtkError::MissingData("polys"))?;
//         let vertices = self.extract_vertices(&piece.points);

//         // 构造虚拟Cells结构
//         // let cells = model::Cells {
//         //     cell_verts: polys.connectivity.clone().into(),
//         //     types: vec![CellType::Polygon; polys.num_cells() as usize],
//         // };

//         // let indices = self.triangulate_cells(cells);

//         // Ok(GeometryData {
//         //     vertices,
//         //     indices,
//         // })
//         todo!()
//     }

//     // Simple implementation
//     fn triangulate_polygon(&self, verts: &[u32]) -> Vec<u32> {
//         let mut indices = Vec::with_capacity((verts.len() - 2) * 3);
//         for i in 1..verts.len() - 1 {
//             indices.push(verts[0]);
//             indices.push(verts[i]);
//             indices.push(verts[i + 1]);
//         }
//         indices
//     }
// }

//************************************* Main Process Logic**************************************//
pub fn process_vtk_file_legacy(path: &PathBuf) -> Result<Mesh, VtkError> {
    let geometry;
    let vtk = Vtk::import(PathBuf::from(format!("{}", path.to_string_lossy())))
        .map_err(|e| VtkError::LoadError(e.to_string()))?;
    match vtk.data {
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            let extractor = UnstructuredGridExtractor;
            geometry = extractor.process_legacy(pieces)?;
        }
        model::DataSet::PolyData { meta, pieces } => {
            // self.process_legacy(pieces)
            todo!()
        }
        _ => {
            return Err(VtkError::UnsupportedDataType);
        }
    }
    // let extractor = MeshExtractor;
    // let geometry = extractor.extract_geometry(vtk.map(|vtk| vtk.data)?)?;

    Ok(create_mesh_legacy(geometry))
}

pub fn create_mesh_legacy(geometry: GeometryData) -> Mesh {
    // initialize a mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // process vertices position attributes
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(geometry.vertices),
    );

    // process vertices indices attributes
    mesh.insert_indices(Indices::U32(geometry.indices));

    // compute normals
    mesh.compute_normals();

    mesh
}

//**************************************************************************//
