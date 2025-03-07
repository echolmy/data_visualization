use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::utils::HashMap;
use std::path::PathBuf;
use vtkio::*;

/***********************************************************
* Error Type Start
***********************************************************/
#[derive(Debug)]
#[allow(dead_code)]
pub enum VtkError {
    LoadError(String),
    InvalidFormat(&'static str),
    UnsupportedDataType,
    MissingData(&'static str),
    IndexOutOfBounds {
        index: usize,
        max: usize,
    },
    DataTypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    AttributeMismatch {
        attribute_size: usize,
        expected_size: usize,
    },
    ConversionError(String),
    IoError(std::io::Error),
    GenericError(String),
}

impl std::fmt::Display for VtkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VtkError::LoadError(msg) => write!(f, "加载VTK文件错误: {}", msg),
            VtkError::InvalidFormat(detail) => write!(f, "VTK格式无效: {}", detail),
            VtkError::UnsupportedDataType => write!(f, "不支持的数据类型"),
            VtkError::MissingData(what) => write!(f, "缺少数据: {}", what),
            VtkError::IndexOutOfBounds { index, max } => {
                write!(f, "索引超出边界: {} (最大值为 {})", index, max)
            }
            VtkError::DataTypeMismatch { expected, found } => {
                write!(f, "数据类型不匹配: 期望 {}, 找到 {}", expected, found)
            }
            VtkError::AttributeMismatch {
                attribute_size,
                expected_size,
            } => {
                write!(
                    f,
                    "属性大小不匹配: 属性大小 {}, 期望 {}",
                    attribute_size, expected_size
                )
            }
            VtkError::ConversionError(msg) => write!(f, "转换错误: {}", msg),
            VtkError::IoError(err) => write!(f, "IO错误: {}", err),
            VtkError::GenericError(msg) => write!(f, "错误: {}", msg),
        }
    }
}

impl std::error::Error for VtkError {}

impl From<std::io::Error> for VtkError {
    fn from(err: std::io::Error) -> Self {
        VtkError::IoError(err)
    }
}

/***********************************************************
* Error Type End
***********************************************************/

/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field
#[derive(Clone)]
pub struct GeometryData {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,

    attributes: Option<HashMap<(String, AttributeLocation), AttributeType>>,
    // normals: Option<Vec<[f32; 3]>>,
}
impl GeometryData {
    pub fn new(
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
        attributes: HashMap<(String, AttributeLocation), AttributeType>,
    ) -> Self {
        Self {
            vertices,
            indices,
            attributes: Some(attributes),
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
    // 新增方法，处理点属性颜色
    pub fn apply_point_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    AttributeLocation::Point => {
                        if let AttributeType::ColorScalar {
                            nvalues: point_nvalues,
                            data: point_data,
                        } = attr
                        {
                            Some((point_nvalues, point_data))
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            if let Some((nvalues, data)) = color_scalar {
                let colors = self.process_point_color_scalars(*nvalues, data)?;
                if !colors.is_empty() {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(colors),
                    );
                    println!("点颜色已插入网格");
                    return Ok(());
                }
            }
        }

        println!("没有找到点颜色属性");
        Ok(())
    }

    fn process_point_color_scalars(
        &self,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Result<Vec<[f32; 4]>, VtkError> {
        if data.len() != self.vertices.len() {
            println!(
                "警告: 颜色数据数量({})与顶点数量({})不匹配",
                data.len(),
                self.vertices.len()
            );
            // 可以选择返回错误或继续处理
        }

        let mut colors = Vec::with_capacity(self.vertices.len());

        for (idx, color_data) in data.iter().enumerate() {
            if idx >= self.vertices.len() {
                break;
            }

            let color = match nvalues {
                3 => [color_data[0], color_data[1], color_data[2], 1.0],
                4 => [color_data[0], color_data[1], color_data[2], color_data[3]],
                _ => [1.0, 1.0, 1.0, 1.0], // 默认白色
            };

            colors.push(color);
        }

        if colors.len() < self.vertices.len() {
            // 如果颜色数据不足，用默认颜色补齐
            colors.resize(self.vertices.len(), [1.0, 1.0, 1.0, 1.0]);
        }

        Ok(colors)
    }

    pub fn apply_cell_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        println!("Attributes status: {:?}", self.attributes.is_some());
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
                println!("Colors inserted into mesh");
            } else {
                println!("No attributes found");
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

    // 处理标量属性
    pub fn apply_scalar_attributes(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            // 查找所有标量属性
            for ((name, location), attr) in attributes {
                if let AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                } = attr
                {
                    println!(
                        "处理标量属性: {}, 位置: {:?}, 组件数: {}",
                        name, location, num_comp
                    );

                    match location {
                        AttributeLocation::Point => {
                            // 点属性，直接插入
                            if *num_comp == 1 {
                                // 对于单一标量，可以考虑转换为颜色
                                let mut colors = Vec::with_capacity(self.vertices.len());

                                // 计算最小最大值以进行归一化
                                let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                                let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                                let range = max_val - min_val;

                                for &val in data.iter() {
                                    let normalized = if range > 0.0 {
                                        (val - min_val) / range
                                    } else {
                                        0.5 // 防止除以0
                                    };

                                    // 使用一个简单的梯度从蓝色到红色
                                    let r = normalized;
                                    let g = 0.2;
                                    let b = 1.0 - normalized;

                                    colors.push([r, g, b, 1.0]);
                                }

                                if colors.len() == self.vertices.len() {
                                    mesh.insert_attribute(
                                        Mesh::ATTRIBUTE_COLOR,
                                        VertexAttributeValues::from(colors),
                                    );
                                    println!("标量已转换为颜色并插入网格");
                                }
                            }
                            // 对于其他组件数，可能需要其他处理方式
                        }
                        AttributeLocation::Cell => {
                            // 单元格属性，需要将每个单元格的值分配给其顶点
                            // 这里可以实现类似process_cell_color_scalars的逻辑
                            println!("单元格标量属性处理待实现");
                        }
                    }
                }
            }
        }

        Ok(())
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
                println!(
                    "处理标量数据: {} 组件, 查找表: {:?}",
                    num_comp, lookup_table
                );

                Ok((
                    name.to_string(),
                    AttributeType::Scalar {
                        num_comp: *num_comp as usize,
                        table_name: lookup_table
                            .clone()
                            .unwrap_or_else(|| "default".to_string()),
                        data: values,
                    },
                ))
            }
            model::ElementType::ColorScalars(nvalues) => {
                let color_values = values
                    .chunks_exact(*nvalues as usize)
                    .map(|v| v.to_vec())
                    .collect();

                Ok((
                    name.to_string(),
                    AttributeType::ColorScalar {
                        nvalues: *nvalues,
                        data: color_values,
                    },
                ))
            }
            model::ElementType::Vectors => {
                // 处理向量类型，每个向量有3个分量(x,y,z)
                let vectors: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(vectors)))
            }
            model::ElementType::Normals => {
                // 处理法线向量，与Vectors类似
                let normals: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(normals)))
            }
            model::ElementType::TCoords(n) => {
                println!("纹理坐标: {} 分量", n);
                // 简单处理为向量类型
                let coords: Vec<[f32; 3]> = if *n == 2 {
                    // 2D纹理坐标，第三个分量为0
                    values
                        .chunks_exact(2)
                        .map(|chunk| [chunk[0], chunk[1], 0.0])
                        .collect()
                } else if *n == 3 {
                    // 3D纹理坐标
                    values
                        .chunks_exact(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                        .collect()
                } else {
                    return Err(VtkError::InvalidFormat("不支持的纹理坐标维度"));
                };

                Ok((name.to_string(), AttributeType::Vector(coords)))
            }
            model::ElementType::Tensors => {
                println!("张量数据暂不完全支持，简化处理");
                // 简化处理为向量集合
                let tensors: Vec<[f32; 3]> = values
                    .chunks_exact(9) // 3x3 张量
                    .map(|chunk| {
                        // 简化: 使用对角线元素
                        [chunk[0], chunk[4], chunk[8]]
                    })
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(tensors)))
            }
            model::ElementType::LookupTable => {
                println!("查找表数据暂不支持直接处理");
                Err(VtkError::UnsupportedDataType)
            }
            model::ElementType::Generic(desc) => {
                println!("通用类型: {:?} 暂不支持完全处理", desc);
                Err(VtkError::UnsupportedDataType)
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
        let tmp: HashMap<(String, AttributeLocation), AttributeType> = HashMap::new();
        Ok(GeometryData::new(vertices, indices, tmp))
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

                model::CellType::Quad => {
                    // 将四边形分解为两个三角形
                    if num_vertices != 4 {
                        panic!("Invalid quad vertex count: {} (expected 4)", num_vertices);
                    }
                    indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                    indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                }

                model::CellType::Tetra => {
                    // 四面体分解为4个三角形
                    panic!("Invalid tetrahedron vertex count.");
                    indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                    indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                    indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
                    indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
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

        let attributes = self.extract_attributes_legacy(&pieces);

        let vertices = self.extract_vertices(&piece.points);
        let indices = self.extract_indices(pieces);

        Ok(GeometryData::new(vertices, indices, attributes?))
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

    fn triangulate_polygon(&self, topology: model::VertexNumbers) -> Vec<u32> {
        let mut indices = Vec::new();
        let poly_data = topology.into_legacy();

        let num_cells = poly_data.0;
        // create iterator
        let mut data_iter = poly_data.1.iter().copied().peekable();

        // iterate over all cells
        for _i in 0..num_cells {
            if data_iter.peek().is_none() {
                // use Result instead of panic
                println!("Warning: incomplete data structure, may cause rendering errors");
                break;
            }

            // load the number of vertices of each cell (first number of each row of cell)
            let num_vertices = match data_iter.next() {
                Some(n) => n as usize,
                None => {
                    println!("Warning: missing vertex count information");
                    break;
                }
            };

            // collect vertices
            let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

            if vertices.len() < 3 {
                // if less than 3 vertices, cannot form a triangle
                continue;
            }

            // choose triangulation strategy based on number of vertices
            match vertices.len() {
                3 => {
                    // already a triangle, add directly
                    indices.extend_from_slice(&vertices);
                }
                4 => {
                    // quadrilateral can be divided into two triangles
                    indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                    indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                }
                _ => {
                    // 一般多边形使用耳切法（ear clipping）或扇形三角化
                    indices.extend(self.triangulate_complex_polygon(&vertices));
                }
            }
        }

        // check if there is still remaining data (this is a warning, not an error)
        if data_iter.next().is_some() {
            println!("Warning: processed data still has additional data remaining, may not be fully parsed");
        }

        indices
    }

    // 改进的多边形三角化算法，使用改进的扇形三角化
    // 尝试找到一个适合作为扇形中心的顶点
    fn triangulate_complex_polygon(&self, vertices: &[u32]) -> Vec<u32> {
        if vertices.len() < 3 {
            return Vec::new();
        }

        // 如果是三角形，直接返回
        if vertices.len() == 3 {
            return vertices.to_vec();
        }

        // 分配空间：对于n个顶点的多边形，需要(n-2)*3个索引
        let mut indices = Vec::with_capacity((vertices.len() - 2) * 3);

        // 对于简单情况，使用首个顶点作为中心点
        // 注意：这种方法对于凹多边形可能会产生错误，但对于大多数VTK文件中的凸多边形应该足够
        let center_vertex = vertices[0];

        // 创建三角形扇形 - 顺时针方向
        for i in 1..vertices.len() - 1 {
            indices.push(center_vertex); // 中心点
            indices.push(vertices[i]); // 当前点
            indices.push(vertices[i + 1]); // 下一个点
        }

        indices
    }
}

//************************************* Main Process Logic**************************************//
// 在process_vtk_file_legacy函数中添加对更多属性的处理
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

    println!("提取的几何数据属性信息: {:?}", &geometry.attributes);

    // 创建带属性的网格
    let mut mesh = create_mesh_legacy(geometry.clone());

    // 应用颜色属性
    let _ = geometry.apply_cell_color_scalars(&mut mesh);

    // 如果没有单元格颜色，尝试应用点颜色
    if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
        let _ = geometry.apply_point_color_scalars(&mut mesh);
    }

    // 应用其他标量属性（如果有）
    let _ = geometry.apply_scalar_attributes(&mut mesh);

    Ok(mesh)
}

pub fn create_mesh_legacy(geometry: GeometryData) -> Mesh {
    // initialize a mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Set color
    println!("{:?}", &geometry.attributes);
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
