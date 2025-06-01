//! # 高阶网格转换模块
//!
//! 此模块专门用于将低阶网格（线性三角形）转换为高阶网格（二阶三角形），
//! 提供网格精度提升功能。
//!
//! ## 核心功能
//!
//! - **升阶转换**：将一阶（线性）网格转换为二阶（二次）网格
//! - **边中点生成**：自动计算和管理网格边的中点顶点
//! - **几何数据保持**：在转换过程中保持原有的属性信息
//!
//! ## 支持的转换类型
//!
//! - **输入**：线性三角形网格（每个三角形3个顶点）
//! - **输出**：二阶三角形网格（每个三角形6个顶点：3个角点 + 3个边中点）
//!
//! ## 使用示例
//!
//! ```rust
//! // 将线性网格转换为二阶网格
//! let higher_order_geometry = convert_to_higher_order(&geometry, 2)?;
//! ```
//!
//! ## 技术细节
//!
//! - 二阶三角形按照VTK标准格式存储：[v0, v1, v2, mid01, mid12, mid20]
//! - 边中点通过HashMap缓存避免重复计算
//! - 生成的高阶网格可通过标准VTK处理流程进行渲染
//!
//! ## 注意事项
//!
//! - 高阶网格的渲染细分由 `triangulation.rs` 模块统一处理
//! - 直接导入的高阶VTK文件会自动在解析阶段完成细分处理
//! - 本模块主要用于从简单网格生成高精度网格的场景
//!
//! ## 限制
//!
//! - 目前仅支持三角形网格（不支持四面体或其他单元类型）
//! - 仅实现了二阶转换（order=2），不支持更高阶
//! - 输入网格必须是有效的三角形网格（索引数量必须是3的倍数）

use super::vtk::*;
use super::VtkError;
use std::collections::HashMap;

/// 将一阶网格转换为高阶网格（主入口函数）
///
/// 这是高阶网格转换的主要接口，支持将线性网格转换为指定阶数的高阶网格。
/// 目前仅支持二阶转换，未来可扩展支持更高阶。
///
/// # 参数
/// * `geometry` - 原始的一阶几何数据
/// * `order` - 目标阶数（目前只支持2=二阶）
///
/// # 返回值
/// * `Ok(GeometryData)` - 转换后的高阶几何数据
/// * `Err(VtkError)` - 转换失败时的错误信息
///
/// # 错误
/// * `InvalidFormat` - 当 order < 2 时
/// * `UnsupportedDataType` - 当 order > 2 时（暂未实现）
///
/// # 示例
/// ```rust
/// let higher_order_mesh = convert_to_higher_order(&linear_geometry, 2)?;
/// ```
pub fn convert_to_higher_order(
    geometry: &GeometryData,
    order: u32,
) -> Result<GeometryData, VtkError> {
    if order < 2 {
        return Err(VtkError::InvalidFormat("Order must be >= 2"));
    }

    println!("Converting linear mesh to order {} mesh", order);

    match order {
        2 => convert_to_second_order(geometry),
        _ => Err(VtkError::UnsupportedDataType),
    }
}

/// 将一阶网格转换为二阶网格
///
/// 执行从线性三角形到二阶三角形的具体转换过程。为每条边生成中点顶点，
/// 并重新组织索引结构以符合VTK二阶三角形格式。同时正确处理属性数据插值。
///
/// # 参数
/// * `geometry` - 原始的一阶几何数据
///
/// # 返回值
/// * `Ok(GeometryData)` - 转换后的二阶几何数据
/// * `Err(VtkError)` - 转换失败时的错误信息
///
/// # 错误
/// * `InvalidFormat` - 当输入网格不是三角形网格时
///
/// # 转换过程
/// 1. 验证输入网格是否为有效的三角形网格
/// 2. 为每条边生成中点顶点（避免重复）
/// 3. 重新组织索引为二阶三角形格式
/// 4. 插值计算新顶点的属性数据
/// 5. 生成新的三角形到单元格映射关系
pub fn convert_to_second_order(geometry: &GeometryData) -> Result<GeometryData, VtkError> {
    println!("Converting linear mesh to quadratic mesh");

    let original_vertices = &geometry.vertices;
    let original_indices = &geometry.indices;

    // 1. 检查是否为三角形网格
    if original_indices.len() % 3 != 0 {
        return Err(VtkError::InvalidFormat("Mesh must be triangular"));
    }

    let num_triangles = original_indices.len() / 3;
    println!(
        "Original mesh: {} vertices, {} triangles",
        original_vertices.len(),
        num_triangles
    );

    // 2. 转换每个线性三角形为二阶三角形
    let (new_vertices, new_indices, edge_midpoint_map) =
        convert_linear_triangles_to_quadratic_with_mapping(original_vertices, original_indices)?;

    println!(
        "Quadratic mesh: {} vertices, {} quadratic triangles",
        new_vertices.len(),
        num_triangles
    );

    // 3. 处理属性数据插值
    let new_attributes = interpolate_attributes_for_higher_order(
        geometry,
        &edge_midpoint_map,
        original_vertices.len(),
        new_vertices.len(),
    )?;

    // 4. 生成新的三角形到单元格映射关系（每个二阶三角形对应4个线性三角形）
    let new_triangle_to_cell_mapping = generate_higher_order_triangle_mapping(num_triangles);

    // 5. 创建新的几何数据
    let new_attributes_converted = new_attributes.into_iter().collect();
    let mut new_geometry = GeometryData::new(new_vertices, new_indices, new_attributes_converted);
    new_geometry.triangle_to_cell_mapping = Some(new_triangle_to_cell_mapping);

    Ok(new_geometry)
}

/// 将线性三角形转换为二阶三角形（带边中点映射）
///
/// 这是转换算法的核心实现。对每个线性三角形，生成三个边中点，
/// 并按照VTK QuadraticTriangle的标准顶点顺序重新组织索引。
/// 同时返回边中点映射关系，用于属性插值。
///
/// # 参数
/// * `vertices` - 原始顶点列表
/// * `indices` - 原始索引列表（线性三角形）
///
/// # 返回值
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>))` - (新顶点列表, 新索引列表, 边中点映射)
/// * `Err(VtkError)` - 转换失败时的错误信息
///
/// # VTK顶点顺序
/// 二阶三角形的6个顶点按以下顺序存储：
/// - [0,1,2]: 三个角点
/// - [3,4,5]: 三个边中点 (edge01, edge12, edge20)
///
/// # 优化特性
/// - 使用HashMap缓存边中点，避免重复计算
/// - 边的表示统一为 (min_vertex, max_vertex) 确保一致性
fn convert_linear_triangles_to_quadratic_with_mapping(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> Result<(Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>), VtkError> {
    let num_triangles = indices.len() / 3;

    // 存储边及其对应的中点索引
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();
    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::new();

    // 为每个线性三角形生成二阶三角形
    for triangle_idx in 0..num_triangles {
        let base_idx = triangle_idx * 3;
        let v0 = indices[base_idx];
        let v1 = indices[base_idx + 1];
        let v2 = indices[base_idx + 2];

        // 获取或创建边中点
        let mid01 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v0, v1);
        let mid12 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v1, v2);
        let mid20 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v2, v0);

        // 按照VTK QuadraticTriangle顶点顺序：[v0, v1, v2, mid01, mid12, mid20]
        // 这与CellType::QuadraticTriangle的标准布局一致
        new_indices.extend_from_slice(&[v0, v1, v2, mid01, mid12, mid20]);
    }

    Ok((new_vertices, new_indices, edge_midpoints))
}

/// 获取或创建边中点
///
/// 管理边中点的生成和缓存，确保相同的边只生成一次中点。
/// 这对于共享边的相邻三角形非常重要，避免了重复顶点的产生。
///
/// # 参数
/// * `edge_midpoints` - 边中点映射表的可变引用
/// * `vertices` - 顶点列表的可变引用
/// * `v0` - 边的第一个顶点索引
/// * `v1` - 边的第二个顶点索引
///
/// # 返回值
/// * `u32` - 边中点的顶点索引
///
/// # 实现细节
/// 1. 统一边的表示方式（较小索引在前）
/// 2. 检查中点是否已存在于缓存中
/// 3. 如不存在，计算中点坐标并添加到顶点列表
/// 4. 更新缓存映射表
fn get_or_create_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
) -> u32 {
    // 1. 确保边顶点顺序的一致性
    // 边由较小顶点索引和较大顶点索引定义
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // 2. 如果中点已存在，直接返回
    // 因为边可能在不同三角形中被重用，所以中点可能被重用
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    // 3. 如果中点不存在，计算中点坐标
    let pos0 = vertices[v0 as usize];
    let pos1 = vertices[v1 as usize];
    let midpoint = [
        (pos0[0] + pos1[0]) * 0.5,
        (pos0[1] + pos1[1]) * 0.5,
        (pos0[2] + pos1[2]) * 0.5,
    ];

    // 4. 添加新顶点
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // 5. 记录边中点映射
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}

/// 为高阶网格插值属性数据
///
/// 对于新生成的边中点顶点，通过线性插值计算其属性值。
/// 确保转换后的高阶网格保持正确的颜色和标量数据。
///
/// # 参数
/// * `geometry` - 原始几何数据
/// * `edge_midpoint_map` - 边中点映射关系
/// * `original_vertex_count` - 原始顶点数量
/// * `new_vertex_count` - 新顶点数量
///
/// # 返回值
/// * `Ok(HashMap)` - 插值后的属性数据
/// * `Err(VtkError)` - 插值失败时的错误信息
fn interpolate_attributes_for_higher_order(
    geometry: &GeometryData,
    edge_midpoint_map: &HashMap<(u32, u32), u32>,
    _original_vertex_count: usize,
    new_vertex_count: usize,
) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
    let mut new_attributes = HashMap::new();

    if let Some(attributes) = &geometry.attributes {
        for ((name, location), attr) in attributes.iter() {
            match location {
                AttributeLocation::Point => {
                    // 处理点属性：需要为新的边中点插值
                    let interpolated_attr = interpolate_point_attribute(
                        attr,
                        edge_midpoint_map,
                        _original_vertex_count,
                        new_vertex_count,
                    )?;
                    new_attributes.insert((name.clone(), location.clone()), interpolated_attr);
                }
                AttributeLocation::Cell => {
                    // 单元格属性保持不变，因为单元格数量没有改变
                    new_attributes.insert((name.clone(), location.clone()), attr.clone());
                }
            }
        }
    }

    Ok(new_attributes)
}

/// 插值点属性数据
///
/// 为新生成的边中点顶点计算属性值，通过线性插值方法。
fn interpolate_point_attribute(
    attr: &AttributeType,
    edge_midpoint_map: &HashMap<(u32, u32), u32>,
    _original_vertex_count: usize,
    new_vertex_count: usize,
) -> Result<AttributeType, VtkError> {
    match attr {
        AttributeType::Scalar {
            num_comp,
            data,
            table_name,
            lookup_table,
        } => {
            let mut new_data = data.clone();
            new_data.resize(new_vertex_count, 0.0);

            // 为每个边中点插值标量值
            for ((v0, v1), &midpoint_idx) in edge_midpoint_map.iter() {
                let val0 = data.get(*v0 as usize).copied().unwrap_or(0.0);
                let val1 = data.get(*v1 as usize).copied().unwrap_or(0.0);
                let interpolated_val = (val0 + val1) * 0.5;

                if (midpoint_idx as usize) < new_data.len() {
                    new_data[midpoint_idx as usize] = interpolated_val;
                }
            }

            Ok(AttributeType::Scalar {
                num_comp: *num_comp,
                data: new_data,
                table_name: table_name.clone(),
                lookup_table: lookup_table.clone(),
            })
        }
        AttributeType::ColorScalar { nvalues, data } => {
            let mut new_data = data.clone();
            new_data.resize(new_vertex_count, vec![1.0; *nvalues as usize]);

            // 为每个边中点插值颜色值
            for ((v0, v1), &midpoint_idx) in edge_midpoint_map.iter() {
                let color0 = data
                    .get(*v0 as usize)
                    .cloned()
                    .unwrap_or(vec![1.0; *nvalues as usize]);
                let color1 = data
                    .get(*v1 as usize)
                    .cloned()
                    .unwrap_or(vec![1.0; *nvalues as usize]);

                let mut interpolated_color = vec![0.0; *nvalues as usize];
                for i in 0..(*nvalues as usize) {
                    let val0 = color0.get(i).copied().unwrap_or(1.0);
                    let val1 = color1.get(i).copied().unwrap_or(1.0);
                    interpolated_color[i] = (val0 + val1) * 0.5;
                }

                if (midpoint_idx as usize) < new_data.len() {
                    new_data[midpoint_idx as usize] = interpolated_color;
                }
            }

            Ok(AttributeType::ColorScalar {
                nvalues: *nvalues,
                data: new_data,
            })
        }
        AttributeType::Vector(data) => {
            let mut new_data = data.clone();
            new_data.resize(new_vertex_count, [0.0, 0.0, 0.0]);

            // 为每个边中点插值向量值
            for ((v0, v1), &midpoint_idx) in edge_midpoint_map.iter() {
                let vec0 = data.get(*v0 as usize).copied().unwrap_or([0.0, 0.0, 0.0]);
                let vec1 = data.get(*v1 as usize).copied().unwrap_or([0.0, 0.0, 0.0]);

                let interpolated_vec = [
                    (vec0[0] + vec1[0]) * 0.5,
                    (vec0[1] + vec1[1]) * 0.5,
                    (vec0[2] + vec1[2]) * 0.5,
                ];

                if (midpoint_idx as usize) < new_data.len() {
                    new_data[midpoint_idx as usize] = interpolated_vec;
                }
            }

            Ok(AttributeType::Vector(new_data))
        }
        AttributeType::Tensor(data) => {
            let mut new_data = data.clone();
            new_data.resize(new_vertex_count, [0.0; 9]);

            // 为每个边中点插值张量值
            for ((v0, v1), &midpoint_idx) in edge_midpoint_map.iter() {
                let tensor0 = data.get(*v0 as usize).copied().unwrap_or([0.0; 9]);
                let tensor1 = data.get(*v1 as usize).copied().unwrap_or([0.0; 9]);

                let mut interpolated_tensor = [0.0; 9];
                for i in 0..9 {
                    interpolated_tensor[i] = (tensor0[i] + tensor1[i]) * 0.5;
                }

                if (midpoint_idx as usize) < new_data.len() {
                    new_data[midpoint_idx as usize] = interpolated_tensor;
                }
            }

            Ok(AttributeType::Tensor(new_data))
        }
    }
}

/// 为高阶网格生成三角形到单元格映射
///
/// 每个二阶三角形在渲染时会被细分为4个线性三角形，
/// 需要生成相应的映射关系以保持颜色渲染的正确性。
fn generate_higher_order_triangle_mapping(num_original_triangles: usize) -> Vec<usize> {
    let mut mapping = Vec::with_capacity(num_original_triangles * 4);

    for cell_idx in 0..num_original_triangles {
        // 每个原始三角形单元格对应4个细分三角形
        for _ in 0..4 {
            mapping.push(cell_idx);
        }
    }

    mapping
}
