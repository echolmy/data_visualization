use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::utils::HashMap;

use super::VtkError;
use crate::mesh::color_maps;
use crate::mesh::triangulation;
use vtkio::*;

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

/// VtkDataset:
/// Structured Points; Structured Grid; Rectilinear Grid; Polygonal Data; Unstructured Grid; Field
#[derive(Clone)]
pub struct GeometryData {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub attributes: Option<HashMap<(String, AttributeLocation), AttributeType>>,
    // normals: Option<Vec<[f32; 3]>>,
    pub triangle_to_cell_mapping: Option<Vec<usize>>,
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
            triangle_to_cell_mapping: None,
        }
    }

    // add attribute data
    pub fn add_attributes(
        mut self,
        attributes: HashMap<(String, AttributeLocation), AttributeType>,
    ) -> Self {
        self.attributes = Some(attributes);
        self
    }

    // add triangle to cell mapping
    pub fn add_triangle_to_cell_mapping(mut self, mapping: Vec<usize>) -> Self {
        self.triangle_to_cell_mapping = Some(mapping);
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

    /// Apply point color scalars to a mesh.
    ///
    /// This function will try to find a `ColorScalar` attribute in the `Point` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the color scalars are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `ColorScalar`.
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
                    println!("Point color scalars inserted into mesh.");
                    return Ok(());
                }
            }
        }

        println!("No point color attribute found.");
        Ok(())
    }

    /// Process point color scalars from attribute data.
    ///
    /// This function takes the number of values and the data for each point,
    /// and returns a vector of colors.
    ///
    /// Returns `Ok(colors)` if the colors are successfully processed,
    /// or `Err(VtkError)` if the data is invalid.
    fn process_point_color_scalars(
        &self,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Result<Vec<[f32; 4]>, VtkError> {
        if data.len() != self.vertices.len() {
            println!(
                "Warning: color data number({}) does not match vertex number({})",
                data.len(),
                self.vertices.len()
            );
            // TODO: could return error or continue processing
        }

        let mut colors = Vec::with_capacity(self.vertices.len());

        for (idx, color_data) in data.iter().enumerate() {
            if idx >= self.vertices.len() {
                break;
            }

            // match color data number (RGBA or RGB)
            let color = match nvalues {
                3 => [color_data[0], color_data[1], color_data[2], 1.0],
                4 => [color_data[0], color_data[1], color_data[2], color_data[3]],
                _ => [1.0, 1.0, 1.0, 1.0], // default white
            };

            colors.push(color);
        }

        // if color data is not enough, use default color to fill
        if colors.len() < self.vertices.len() {
            colors.resize(self.vertices.len(), [1.0, 1.0, 1.0, 1.0]);
        }

        Ok(colors)
    }

    /// Apply cell color scalars to a mesh.
    ///
    /// This function will try to find a `ColorScalar` attribute in the `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the color scalars are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `ColorScalar`.
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
                self.process_cell_color_scalars_internal(*nvalues, data)
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

    /// Process cell color scalars from attribute data.
    ///
    /// This function takes the number of values and the data for each cell,
    /// and returns a vector of colors.
    ///
    /// Returns `Ok(colors)` if the colors are successfully processed,
    /// or `Err(VtkError)` if the data is invalid.
    fn process_cell_color_scalars_internal(
        &self,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Vec<[f32; 4]> {
        // initialize color list for each vertex (white)
        let mut vertices_color = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

        // 使用映射关系来确保正确映射
        if let Some(mapping) = &self.triangle_to_cell_mapping {
            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                // 检查cell_idx是否有效
                if cell_idx >= data.len() {
                    println!(
                        "Warning: cell index {} exceeds color data range {}",
                        cell_idx,
                        data.len()
                    );
                    continue;
                }

                // 获取这个三角形的三个顶点索引
                let triangle_base = triangle_idx * 3;
                if triangle_base + 2 >= self.indices.len() {
                    println!(
                        "Warning: triangle index {} exceeds index range {}",
                        triangle_base,
                        self.indices.len()
                    );
                    continue;
                }

                let vertex_indices = [
                    self.indices[triangle_base] as usize,
                    self.indices[triangle_base + 1] as usize,
                    self.indices[triangle_base + 2] as usize,
                ];

                // 获取对应单元格的颜色
                let colors = &data[cell_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                };

                // 设置顶点颜色
                for &idx in &vertex_indices {
                    if idx < vertices_color.len() {
                        vertices_color[idx] = color;
                    }
                }
            }
        } else {
            // 回退到原始方法，按顺序一一对应 (旧代码保留为后备)
            let num_triangles = self.indices.len() / 3;
            for triangle_idx in 0..num_triangles {
                if triangle_idx >= data.len() {
                    println!(
                        "Warning: triangle index {} exceeds color data range {}",
                        triangle_idx,
                        data.len()
                    );
                    break;
                }

                // 获取这个三角形的三个顶点索引
                let vertex_indices = [
                    self.indices[triangle_idx * 3] as usize,
                    self.indices[triangle_idx * 3 + 1] as usize,
                    self.indices[triangle_idx * 3 + 2] as usize,
                ];

                // 获取这个cell的颜色
                let colors = &data[triangle_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                };

                // 设置顶点颜色
                for &idx in &vertex_indices {
                    if idx < vertices_color.len() {
                        vertices_color[idx] = color;
                    }
                }
            }
        }

        vertices_color
    }

    /// Apply scalar attributes to a mesh.
    ///
    /// This function will try to find a `Scalar` attribute in the `Point` or `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the scalar attributes are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `Scalar`.
    pub fn apply_scalar_attributes(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            // 首先处理点标量
            for ((name, location), attr) in attributes.iter() {
                if let AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                } = attr
                {
                    if location == &AttributeLocation::Point && *num_comp == 1 {
                        println!("处理点标量属性: {} 查找表: {}", name, table_name);

                        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

                        // 计算最小最大值以进行归一化
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        let range = max_val - min_val;

                        println!(
                            "标量数据范围: min={}, max={}, range={}",
                            min_val, max_val, range
                        );

                        // 获取颜色映射表
                        let color_map = color_maps::get_color_map(table_name);
                        println!(
                            "使用颜色映射表: {} (颜色数量: {})",
                            color_map.name,
                            color_map.colors.len()
                        );

                        // 为每个顶点设置颜色
                        for (i, &val) in data.iter().enumerate() {
                            if i < self.vertices.len() {
                                let normalized = if range > 0.0 {
                                    (val - min_val) / range
                                } else {
                                    0.5 // 防止除以0
                                };
                                println!("顶点 {}: 原始值={}, 归一化值={}", i, val, normalized);
                                vertex_colors[i] = color_map.get_color(normalized);
                                println!("    颜色: {:?}", vertex_colors[i]);
                            }
                        }

                        // 插入颜色属性
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            VertexAttributeValues::from(vertex_colors),
                        );
                        println!("点标量颜色已插入网格");
                        return Ok(());
                    }
                }
            }

            // 如果没有点标量，则处理单元格标量
            for ((name, location), attr) in attributes.iter() {
                if let AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                } = attr
                {
                    if location == &AttributeLocation::Cell && *num_comp == 1 {
                        println!("处理单元格标量属性: {} 查找表: {}", name, table_name);

                        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

                        // 计算最小最大值以进行归一化
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        let range = max_val - min_val;

                        // 获取颜色映射表
                        let color_map = color_maps::get_color_map(table_name);
                        println!("使用颜色映射表: {}", color_map.name);

                        // 使用映射关系
                        if let Some(mapping) = &self.triangle_to_cell_mapping {
                            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                                // 检查cell_idx是否有效
                                if cell_idx >= data.len() {
                                    println!(
                                        "警告: 单元格索引 {} 超出标量数据范围 {}",
                                        cell_idx,
                                        data.len()
                                    );
                                    continue;
                                }

                                // 获取单元格的标量值并归一化
                                let val = data[cell_idx];
                                let normalized = if range > 0.0 {
                                    (val - min_val) / range
                                } else {
                                    0.5 // 防止除以0
                                };

                                // 使用颜色映射表获取颜色
                                let color = color_map.get_color(normalized);

                                // 获取这个三角形的三个顶点索引
                                let triangle_base = triangle_idx * 3;
                                if triangle_base + 2 >= self.indices.len() {
                                    println!(
                                        "警告: 三角形索引 {} 超出索引范围 {}",
                                        triangle_base,
                                        self.indices.len()
                                    );
                                    continue;
                                }

                                let vertex_indices = [
                                    self.indices[triangle_base] as usize,
                                    self.indices[triangle_base + 1] as usize,
                                    self.indices[triangle_base + 2] as usize,
                                ];

                                // 设置顶点颜色
                                for &idx in &vertex_indices {
                                    if idx < vertex_colors.len() {
                                        vertex_colors[idx] = color;
                                    }
                                }
                            }
                        }

                        // 插入颜色属性
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            VertexAttributeValues::from(vertex_colors),
                        );
                        println!("单元格标量颜色已插入网格");
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }
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
                    "Processing scalar data: {} components, lookup table: {:?}",
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
                println!("Texture coordinates: {} components", n);
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
                    return Err(VtkError::InvalidFormat(
                        "Unsupported texture coordinate dimension",
                    ));
                };

                Ok((name.to_string(), AttributeType::Vector(coords)))
            }
            model::ElementType::Tensors => {
                println!("Tensor data is not fully supported, simplified processing");
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
                println!("Lookup table data is not supported, simplified processing");
                Err(VtkError::UnsupportedDataType)
            }
            model::ElementType::Generic(desc) => {
                println!("Generic type: {:?} is not fully supported", desc);
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
        if let Some(model::Piece::Inline(piece)) = pieces.into_iter().next() {
            let (indices, _) = self.triangulate_cells(piece.cells);
            indices
        } else {
            // 如果没有内联数据，返回空向量
            Vec::new()
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

        // 获取索引和映射表
        let (indices, triangle_to_cell_mapping) =
            if let Some(model::Piece::Inline(p)) = pieces.into_iter().next() {
                self.triangulate_cells(p.cells)
            } else {
                (Vec::new(), Vec::new())
            };

        // use bevy interface to compute normals
        // let normals = compute_normals(&vertices, &indices);
        let tmp: HashMap<(String, AttributeLocation), AttributeType> = HashMap::new();
        let mut geometry = GeometryData::new(vertices, indices, tmp);
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);
        Ok(geometry)
    }
}
impl UnstructuredGridExtractor {
    // general triangulate cells - 使用通用三角化函数
    fn triangulate_cells(&self, cells: model::Cells) -> (Vec<u32>, Vec<usize>) {
        triangulation::triangulate_cells(cells)
    }
}

impl VtkMeshExtractor for PolyDataExtractor {
    type PieceType = Vec<model::Piece<model::PolyDataPiece>>;
    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32> {
        if let Ok((indices, _)) = self.process_polydata(pieces) {
            indices
        } else {
            Vec::new()
        }
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

        // 处理点属性
        if !point_attr_list.is_empty() {
            for point_attr in point_attr_list {
                match point_attr {
                    model::Attribute::DataArray(data_array) => {
                        if let Ok((name, attr_type)) = self.process_data_array(
                            &data_array.name,
                            &data_array.elem,
                            &data_array.data,
                        ) {
                            attributes.insert((name, AttributeLocation::Point), attr_type);
                        }
                    }
                    model::Attribute::Field {
                        name: _,
                        data_array: _,
                    } => {
                        println!("警告: Field属性暂不支持");
                    }
                }
            }
        }

        // 处理单元格属性
        if !cell_attr_list.is_empty() {
            for cell_attr in cell_attr_list {
                match cell_attr {
                    model::Attribute::DataArray(data_array) => {
                        if let Ok((name, attr_type)) = self.process_data_array(
                            &data_array.name,
                            &data_array.elem,
                            &data_array.data,
                        ) {
                            attributes.insert((name, AttributeLocation::Cell), attr_type);
                        }
                    }
                    model::Attribute::Field {
                        name: _,
                        data_array: _,
                    } => {
                        // todo!("Field属性暂不支持")
                        println!("警告: Field属性暂不支持");
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

        // 获取索引和映射表
        let (indices, triangle_to_cell_mapping) = self.process_polydata(pieces.clone())?;

        let mut geometry = GeometryData::new(vertices, indices, attributes?);
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);
        Ok(geometry)
    }
}

impl PolyDataExtractor {
    // 改进的多边形数据处理方法
    fn process_polydata(
        &self,
        pieces: Vec<model::Piece<model::PolyDataPiece>>,
    ) -> Result<(Vec<u32>, Vec<usize>), VtkError> {
        let piece = pieces
            .into_iter()
            .next()
            .ok_or(VtkError::MissingData("未找到片段"))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("需要内联数据"));
        };

        let mut indices = Vec::<u32>::new();
        let mut triangle_to_cell_mapping = Vec::<usize>::new();

        // 处理顶点拓扑（跳过，因为它们不形成表面）
        if let Some(_) = piece.verts {
            println!("发现顶点图元 - 跳过，因为它们不形成表面");
        }

        // 处理线拓扑（跳过，因为它们不形成表面）
        if let Some(_) = piece.lines {
            println!("发现线图元 - 跳过，因为它们不形成表面");
        }

        // 处理多边形拓扑 - 主要处理逻辑
        if let Some(polys) = piece.polys {
            let (polys_indices, polys_mapping) = self.triangulate_polygon(polys);
            indices.extend(polys_indices);
            triangle_to_cell_mapping.extend(polys_mapping);
        }

        // 处理三角形条带（暂不实现）
        if let Some(_strips) = piece.strips {
            println!("发现三角形条带 - 暂不支持");
            // todo!()
        }

        if indices.is_empty() {
            return Err(VtkError::MissingData("片段中未找到表面几何"));
        }

        Ok((indices, triangle_to_cell_mapping))
    }

    fn triangulate_polygon(&self, topology: model::VertexNumbers) -> (Vec<u32>, Vec<usize>) {
        // 使用通用三角化模块中的函数
        triangulation::triangulate_polygon(topology)
    }
}
