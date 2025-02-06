use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::utils::HashMap;
use std::path::PathBuf;
use vtkio::*;

/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field
pub struct GeometryData {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,

    attributes: Option<HashMap<(String, AttributeLocation), AttributeType>>,
    // normals: Option<Vec<[f32; 3]>>,
}
impl GeometryData {
    pub fn new(vertices: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
            attributes: None,
        }
    }

    // add attribute data
    pub fn with_attributes(
        mut self,
        attributes: HashMap<(String, AttributeLocation), AttributeType>,
    ) -> Self {
        self.attributes = Some(attributes);
        self
    }

    // get attribute data
    pub fn get_attributes(
        &self,
        name: &str,
        location: AttributeLocation,
    ) -> Option<&AttributeType> {
        self.attributes.as_ref()?.get(&(name.to_string(), location))
    }

    // // get all keys of available attributes
    // pub fn get_available_attributes(&self) -> Vec<(String, AttributeLocation)> {
    //     self.attributes
    //         .as_ref()
    //         .map(|attrs| attrs.keys().cloned().collect())
    //         .unwrap_or_default()
    // }

    pub fn apply_cell_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    // Get `ColorScalar` data in `Cell` (Only one pair in HashMap).
                    AttributeLocation::Cell => {
                        if let AttributeType::ColorScalar {
                            nvalues: cell_nvalues,
                            data: cell_data,
                        } = attr
                        {
                            Some((cell_nvalues, cell_data))
                        }
                        // No `ColorScalar` data in `Cell`
                        else {
                            None
                        }
                    }
                    // this function does not consider `color scalars in `Point`
                    _ => None,
                });

            // Get vertices color
            let vertices_color = if let Some((nvalues, data)) = color_scalar {
                self.process_cell_color_scalars(*nvalues, data)
            } else {
                Vec::<[f32; 4]>::new()
            };

            // Insert color attributes into Mesh
            if vertices_color.len() != 0 {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    VertexAttributeValues::from(vertices_color),
                );
            }
        }
        Ok(())
    }

    // Only support file which cell topology are all triangles
    fn process_cell_color_scalars(&self, nvalues: u32, data: &Vec<Vec<f32>>) -> Vec<[f32; 4]> {
        // initialize color list for each vertex (white)
        let mut vertices_color = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

        for (triangle_idx, colors) in data.iter().enumerate() {
            // 获取这个三角形的三个顶点索引
            let vertex_indices = [
                self.indices[triangle_idx * 3] as usize,
                self.indices[triangle_idx * 3 + 1] as usize,
                self.indices[triangle_idx * 3 + 2] as usize,
            ];

            // 获取这个cell的颜色
            let color = match nvalues {
                3 => [colors[0], colors[1], colors[2], 1.0],
                4 => [colors[0], colors[1], colors[2], colors[3]],
                _ => [1.0, 1.0, 1.0, 1.0], // default color white
            };

            // 设置顶点颜色
            for idx in vertex_indices {
                vertices_color[idx] = color;
            }
        }
        vertices_color
    }
}
// Definition of type of attributes
#[derive(Debug, Clone)]
pub enum AttributeType {
    // String: table name. If not specified, table name should be `default`
    Scalar {
        num_comp: usize,
        table_name: String,
        data: Vec<f32>,
    },
    ColorScalar {
        nvalues: u32,
        data: Vec<Vec<f32>>,
    },
    Vector(Vec<[f32; 3]>),
    // Tensor
}

// Position of Attribute
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeLocation {
    Point,
    Cell,
}

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

    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError>;

    // basic geometry process
    fn extract_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]> {
        // process point position
        let points = points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        // construct position of each vertex
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
    }

    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32>;

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError>;

    fn process_data_array(
        &self,
        name: &str,
        elem_type: &model::ElementType,
        data: &IOBuffer,
    ) -> Result<(String, AttributeType), VtkError> {
        let values = data.cast_into::<f32>().unwrap();
        match elem_type {
            model::ElementType::Scalars {
                num_comp,
                lookup_table,
            } => {
                println!("{}", num_comp);
                println!("{:?}", lookup_table);
                todo!()
            }
            model::ElementType::ColorScalars(nvalues) => {
                let _values = values
                    .chunks_exact(*nvalues as usize)
                    .map(|v| v.to_vec())
                    .collect();

                Ok((
                    name.to_string(),
                    AttributeType::ColorScalar {
                        nvalues: *nvalues,
                        data: _values,
                    },
                ))
            }
            model::ElementType::Vectors => {
                todo!()
            }
            model::ElementType::LookupTable => {
                todo!()
            }
            model::ElementType::TCoords(_) => {
                // todo!()
                Ok((
                    "todo".to_string(),
                    AttributeType::Vector(Vec::<[f32; 3]>::new()),
                ))
            }
            model::ElementType::Tensors => {
                // todo!()
                Ok((
                    "todo".to_string(),
                    AttributeType::Vector(Vec::<[f32; 3]>::new()),
                ))
            }
            model::ElementType::Normals => {
                todo!()
            }
            model::ElementType::Generic(_) => {
                todo!()
            }
        }
    }
}

pub struct UnstructuredGridExtractor;
pub struct PolyDataExtractor;

impl VtkMeshExtractor for UnstructuredGridExtractor {
    type PieceType = Vec<model::Piece<model::UnstructuredGridPiece>>;

    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
        todo!()
    }
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
            .ok_or(VtkError::MissingData("No pieces found"))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data"));
        };

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(pieces);

        // use bevy interface to compute normals
        // let normals = compute_normals(&vertices, &indices);

        Ok(GeometryData::new(vertices, indices))
    }
}
impl UnstructuredGridExtractor {
    // general triangulate cells
    fn triangulate_cells(&self, cells: model::Cells) -> Vec<u32> {
        // allocate memory according to triangle initially, if small, it will re-allocate
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
    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
        let mut attributes = HashMap::new();

        let model::Piece::Inline(piece) = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found"))?
        else {
            return Err(VtkError::InvalidFormat("Expected inline data"));
        };

        let point_attr_list = &piece.data.point;
        let cell_attr_list = &piece.data.cell;

        // process point attributes
        if point_attr_list.len() != 0 {
            for point_attr in point_attr_list {
                match point_attr {
                    model::Attribute::DataArray(data_array) => {
                        let attribute = self.process_data_array(
                            &data_array.name,
                            &data_array.elem,
                            &data_array.data,
                        );
                        println!("{:?}", attribute);
                        todo!()
                    }
                    model::Attribute::Field { name, data_array } => {
                        todo!("to be continued: {:?} and {:?}", name, data_array)
                    }
                }
            }
        }

        // process cell attributes
        if cell_attr_list.len() != 0 {
            for cell_attr in cell_attr_list {
                match cell_attr {
                    model::Attribute::DataArray(data_array) => {
                        let name = &data_array.name;
                        let elem = &data_array.elem;
                        let data = &data_array.data;
                        let attribute = self.process_data_array(name, elem, data);

                        // let _ = attribute.map(|(name, attr_type)| {
                        //     attributes.insert((name, AttributeLocation::Cell), attr_type);
                        // });
                        attributes.insert(
                            (data_array.name.clone(), AttributeLocation::Cell),
                            attribute?.1,
                        );
                        println!("{:?}", attributes);
                    }
                    model::Attribute::Field { name, data_array } => {
                        todo!("to be continued: {:?} and {:?}", name, data_array)
                    }
                }
            }
        }
        Ok(attributes)
    }
    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };

        let _ = self.extract_attributes_legacy(&pieces);

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(pieces);

        Ok(GeometryData::new(vertices, indices))
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
            .ok_or(VtkError::MissingData("No pieces found"))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data"));
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
    let geometry: GeometryData;
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

    // Set color
    let _ = geometry.apply_cell_color_scalars(&mut mesh);

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
