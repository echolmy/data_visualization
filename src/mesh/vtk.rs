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

pub struct VtkAttribute {}

#[derive(Debug)]
#[allow(dead_code)]
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

    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32>;

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError>;
}

pub struct UnstructuredGridExtractor;
pub struct PolyDataExtractor;

impl VtkMeshExtractor for UnstructuredGridExtractor {
    type PieceType = Vec<model::Piece<model::UnstructuredGridPiece>>;
    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32> {
        if let model::Piece::Inline(piece) = pieces.into_iter().next().unwrap() {
            self.triangulate_cells(piece.cells)
        } else {
            todo!()
        }
    }

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(pieces);

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

impl VtkMeshExtractor for PolyDataExtractor {
    type PieceType = Vec<model::Piece<model::PolyDataPiece>>;
    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32> {
        self.process_polydata(pieces).unwrap()
    }
    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(pieces);

        Ok(GeometryData { vertices, indices })
    }
}

impl PolyDataExtractor {
    fn process_polydata(
        &self,
        pieces: Vec<model::Piece<model::PolyDataPiece>>,
    ) -> Result<Vec<u32>, VtkError> {
        let piece = pieces
            .into_iter()
            .next()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };
        // let vertices = self.extract_vertices(&piece.points);

        let mut indices = Vec::<u32>::new();

        // vertices topology
        if let Some(_) = piece.verts {
            println!("Found vertex primitives - skipping as they don't form surfaces");
        }

        // lines topology
        if let Some(_) = piece.lines {
            println!("Found line primitives - skipping as they don't form surfaces");
        }
        // polygon topology
        if let Some(polys) = piece.polys {
            let polys_indices = self.triangulate_polygon(polys);
            indices.extend(polys_indices);
        }

        if let Some(strips) = piece.strips {
            todo!(
                "implement a function that input is triangle strips, implementing triangulate{:?}",
                strips
            );
        }

        if indices.is_empty() {
            return Err(VtkError::MissingData(
                "No surface geometry found in the piece",
            ));
        }

        Ok(indices)
        // Ok(GeometryData { vertices, indices })
    }

    // Simple implementation
    fn triangulate_polygon(&self, topology: model::VertexNumbers) -> Vec<u32> {
        let mut indices = Vec::new();
        let poly_data = topology.into_legacy();

        let num_cells = poly_data.0;
        // create iterator
        let mut data_iter = poly_data.1.iter().copied().peekable();

        // iterate over all cells
        for _i in 0..num_cells {
            if data_iter.peek().is_none() {
                panic!("Cell type list longer than available data");
            }
            // load the number of each cell (first number of each row of cell)
            // one row starting with numPoints of [label] `POLYGONS`
            let num_vertices = data_iter.next().expect("Missing vertex count") as usize;
            let vertices = data_iter.by_ref().take(num_vertices).collect::<Vec<u32>>();

            indices.extend(self.triangulate_fan(&vertices));
        }
        indices
    }

    fn triangulate_fan(&self, vertices: &[u32]) -> Vec<u32> {
        // 如果顶点少于3个，无法形成三角形
        if vertices.len() < 3 {
            return Vec::new();
        }
        if vertices.len() == 3 {
            return vertices.to_vec();
        }
        // 分配空间：对于n个顶点的多边形，需要(n-2)*3个索引来存储三角形
        let mut indices = Vec::with_capacity((vertices.len() - 2) * 3);
        // 使用第一个顶点作为扇形的中心点
        let center_vertex = vertices[0];
        // 创建三角形扇形
        for i in 1..vertices.len() - 1 {
            indices.push(center_vertex); // 中心点
            indices.push(vertices[i]); // 当前点
            indices.push(vertices[i + 1]); // 下一个点
        }

        indices
    }
}

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
        model::DataSet::PolyData { meta: _, pieces } => {
            let extractor = PolyDataExtractor;
            geometry = extractor.process_legacy(pieces)?;
        }
        _ => {
            return Err(VtkError::UnsupportedDataType);
        }
    }

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
