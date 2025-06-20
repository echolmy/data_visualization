//! # 自适应网格细分模块
//!
//! 本模块为三角网格提供无限细分能力，支持线性三角形（3个顶点）和二次三角形（6个顶点）。
//!
//! ## 核心功能
//!
//!
//!
//! ## 使用示例
//!
//! ```rust
//! // 对网格进行一次细分
//! let subdivided_geometry = subdivide_mesh(&geometry)?;
//! ```

use super::vtk::*;
use super::VtkError;
use std::collections::HashMap;

/// 对网格进行细分
///
/// 这是主要的细分接口，对三角网格执行一级细分。
///
/// # 参数
/// * `geometry` - 要细分的几何数据
///
/// # 返回值
/// * `Ok(GeometryData)` - 细分后的几何数据
/// * `Err(VtkError)` - 如果细分失败，返回错误信息
///
pub fn subdivide_mesh(geometry: &GeometryData) -> Result<GeometryData, VtkError> {
    let original_vertices = &geometry.vertices;
    let original_indices = &geometry.indices;

    // 验证输入
    if original_indices.len() % 3 != 0 {
        return Err(VtkError::InvalidFormat("网格必须是三角形"));
    }

    let num_triangles = original_indices.len() / 3;

    println!(
        "开始网格细分，原始网格: {} 个顶点, {} 个三角形",
        geometry.vertices.len(),
        num_triangles
    );

    // 检测三角形类型并执行适当的细分
    let (new_vertices, new_indices, edge_midpoint_map) =
        smooth_4_subdivision(original_vertices, original_indices)?;

    // 插值属性数据
    let new_attributes = interpolate_attributes_for_subdivision(
        geometry,
        &edge_midpoint_map,
        original_vertices.len(),
        new_vertices.len(),
    )?;

    // 生成新的三角形到单元格映射
    let new_triangle_to_cell_mapping =
        if let Some(original_mapping) = &geometry.triangle_to_cell_mapping {
            generate_subdivided_triangle_mapping(original_mapping)
        } else {
            // 如果原始几何数据没有映射，生成默认映射
            generate_default_triangle_mapping(num_triangles)
        };

    // 创建新的几何数据
    let new_attributes_converted = new_attributes.into_iter().collect();
    let mut new_geometry = GeometryData::new(new_vertices, new_indices, new_attributes_converted);
    new_geometry.triangle_to_cell_mapping = Some(new_triangle_to_cell_mapping);

    println!(
        "细分完成: {} 个顶点, {} 个三角形",
        new_geometry.vertices.len(),
        new_geometry.indices.len() / 3
    );

    Ok(new_geometry)
}

/// 二次插值函数 - 计算任意重心坐标下的点
///
/// 使用6节点二次三角形的插值权重函数计算指定重心坐标处的点坐标。
/// 这是有限元分析中标准的二次三角形插值公式。
///
/// # 参数
/// * `v0, v1, v2` - 三角形的三个角顶点坐标
/// * `m01, m12, m20` - 三角形的三个边中点坐标
/// * `xi, eta` - 重心坐标 (ξ, η)
///
/// # 返回值
/// * `[f32; 3]` - 插值后的点坐标
///
/// # 插值公式
/// P(ξ,η) = N1*v0 + N2*v1 + N3*v2 + N4*m01 + N5*m12 + N6*m20
/// 其中：
/// - N1 = (1-ξ-η)*(1-2ξ-2η)
/// - N2 = ξ*(2ξ-1)
/// - N3 = η*(2η-1)
/// - N4 = 4*ξ*(1-ξ-η)
/// - N5 = 4*ξ*η
/// - N6 = 4*η*(1-ξ-η)
fn quadratic_interpolate(
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    m01: [f32; 3],
    m12: [f32; 3],
    m20: [f32; 3],
    xi: f32,
    eta: f32,
) -> [f32; 3] {
    // 二次插值权重函数
    let n1 = (1.0 - xi - eta) * (1.0 - 2.0 * xi - 2.0 * eta);
    let n2 = xi * (2.0 * xi - 1.0);
    let n3 = eta * (2.0 * eta - 1.0);
    let n4 = 4.0 * xi * (1.0 - xi - eta);
    let n5 = 4.0 * xi * eta;
    let n6 = 4.0 * eta * (1.0 - xi - eta);

    [
        n1 * v0[0] + n2 * v1[0] + n3 * v2[0] + n4 * m01[0] + n5 * m12[0] + n6 * m20[0],
        n1 * v0[1] + n2 * v1[1] + n3 * v2[1] + n4 * m01[1] + n5 * m12[1] + n6 * m20[1],
        n1 * v0[2] + n2 * v1[2] + n3 * v2[2] + n4 * m01[2] + n5 * m12[2] + n6 * m20[2],
    ]
}

/// 二次三角形中心点插值
///
/// 计算二次三角形重心处的点坐标。
/// 重心坐标是 (1/3, 1/3, 1/3)，这是三角形的几何中心。
///
/// # 参数
/// * `v0, v1, v2` - 三角形的三个角顶点坐标
/// * `m01, m12, m20` - 三角形的三个边中点坐标
///
/// # 返回值
/// * `[f32; 3]` - 三角形中心的坐标
fn quadratic_triangle_center(
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    m01: [f32; 3],
    m12: [f32; 3],
    m20: [f32; 3],
) -> [f32; 3] {
    // 重心坐标 (1/3, 1/3, 1/3)
    quadratic_interpolate(v0, v1, v2, m01, m12, m20, 1.0 / 3.0, 1.0 / 3.0)
}

/// 获取或创建二次插值边中点
///
/// 使用二次插值公式计算边的中点坐标，而不是简单的线性插值。
/// 这确保了新生成的点更好地近似原始二次曲面。
///
/// # 参数
/// * `edge_midpoints` - 边中点缓存HashMap
/// * `vertices` - 顶点列表的可变引用
/// * `v0, v1` - 边的两个端点索引
/// * `v0_pos, v1_pos` - 边的两个端点坐标
/// * `m01_pos, m12_pos, m20_pos` - 二次插值所需的边中点坐标
///
/// # 返回值
/// * `u32` - 边中点的顶点索引
fn get_or_create_quadratic_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
    // 二次插值参数 - 假设边中点对应二次插值
    v0_pos: [f32; 3],
    v1_pos: [f32; 3],
    m01_pos: [f32; 3],
    m12_pos: [f32; 3],
    m20_pos: [f32; 3],
) -> u32 {
    // 确保一致的边顶点顺序
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // 如果中点已存在，直接返回
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    // 用二次插值计算边中点
    // 对于边v0-v1的中点，重心坐标为(0.5, 0.0, 0.5)
    let midpoint = quadratic_interpolate(
        v0_pos,
        v1_pos,
        [0.0, 0.0, 0.0],
        m01_pos,
        m12_pos,
        m20_pos,
        0.5,
        0.0,
    );

    // 添加新顶点
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // 记录边中点映射
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}

/// 执行二次三角形细分算法
///
/// 对于二次三角形（6个顶点），此函数实现正确的二次曲面离散化。
/// 每个二次三角形分解为4个线性三角形，更好地近似二次曲面。
///
/// # 参数
/// * `vertices` - 原始顶点列表（每个三角形6个顶点）
/// * `indices` - 原始索引列表
///
/// # 返回值
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>))` - (新顶点列表, 新索引列表, 边中点映射)
/// * `Err(VtkError)` - 如果细分失败，返回错误信息
///
/// # 二次细分策略
/// 每个二次三角形 (v0, v1, v2, m01, m12, m20) 变成4个线性三角形：
/// 1. 中心三角形: (m01, m12, m20)
/// 2. 角三角形1: (v0, m01, m20)
/// 3. 角三角形2: (v1, m12, m01)
/// 4. 角三角形3: (v2, m20, m12)
///
/// 这保持了二次曲面特性，同时创建可以高效渲染的线性三角形。
fn subdivide_quadratic_mesh_internal(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> Result<(Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>), VtkError> {
    let num_triangles = indices.len() / 3;

    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::with_capacity(num_triangles * 4 * 3); // 每个三角形变成4个
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

    for triangle_idx in 0..num_triangles {
        let base_idx = triangle_idx * 3;
        let v0 = indices[base_idx];
        let v1 = indices[base_idx + 1];
        let v2 = indices[base_idx + 2];
        let v0p = vertices[v0 as usize];
        let v1p = vertices[v1 as usize];
        let v2p = vertices[v2 as usize];

        // 计算二次插值的边中点（假设这些是二次三角形的边中点）
        // 这里我们需要估计边中点，或者从原始数据中获取
        // 暂时用线性插值作为估计，但实际应该从二次三角形数据中获取
        let m01p = [
            (v0p[0] + v1p[0]) * 0.5,
            (v0p[1] + v1p[1]) * 0.5,
            (v0p[2] + v1p[2]) * 0.5,
        ];
        let m12p = [
            (v1p[0] + v2p[0]) * 0.5,
            (v1p[1] + v2p[1]) * 0.5,
            (v1p[2] + v2p[2]) * 0.5,
        ];
        let m20p = [
            (v2p[0] + v0p[0]) * 0.5,
            (v2p[1] + v0p[1]) * 0.5,
            (v2p[2] + v0p[2]) * 0.5,
        ];

        // 获取/创建二次插值边中点
        let mid01 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            v0,
            v1,
            v0p,
            v1p,
            m01p,
            m12p,
            m20p,
        );
        let mid12 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            v1,
            v2,
            v1p,
            v2p,
            m01p,
            m12p,
            m20p,
        );
        let mid20 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            v2,
            v0,
            v2p,
            v0p,
            m01p,
            m12p,
            m20p,
        );

        let m01p_new = new_vertices[mid01 as usize];
        let m12p_new = new_vertices[mid12 as usize];
        let m20p_new = new_vertices[mid20 as usize];

        // 用二次插值公式算中心点
        let center = quadratic_triangle_center(v0p, v1p, v2p, m01p_new, m12p_new, m20p_new);
        let center_idx = new_vertices.len() as u32;
        new_vertices.push(center);

        // 4个小三角形
        new_indices.extend_from_slice(&[v0, mid01, center_idx]);
        new_indices.extend_from_slice(&[mid01, v1, center_idx]);
        new_indices.extend_from_slice(&[v1, mid12, center_idx]);
        new_indices.extend_from_slice(&[mid12, v2, center_idx]);
        new_indices.extend_from_slice(&[v2, mid20, center_idx]);
        new_indices.extend_from_slice(&[mid20, v0, center_idx]);
    }
    Ok((new_vertices, new_indices, edge_midpoints))
}

/// 标准4分细分 - 纯线性插值，1->4细分
///
/// 实现标准的4分细分算法，每个三角形分成4个小三角形。
/// 使用纯线性插值计算边中点，保持网格的线性特性。
///
/// # 参数
/// * `vertices` - 原始顶点列表
/// * `indices` - 原始索引列表
///
/// # 返回值
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>))` - (新顶点列表, 新索引列表, 边中点映射)
/// * `Err(VtkError)` - 如果细分失败，返回错误信息
///
/// # 细分策略
/// 每个三角形分成4个小三角形：
/// 1. (v0, mid01, mid20) - 左上角三角形
/// 2. (mid01, v1, mid12) - 右下角三角形
/// 3. (mid20, mid12, v2) - 左下角三角形
/// 4. (mid01, mid12, mid20) - 中心三角形
fn smooth_4_subdivision(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> Result<(Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>), VtkError> {
    let num_triangles = indices.len() / 3;
    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::with_capacity(num_triangles * 4 * 3); // 每个三角形变成4个
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

    for triangle_idx in 0..num_triangles {
        let base_idx = triangle_idx * 3;
        let v0 = indices[base_idx];
        let v1 = indices[base_idx + 1];
        let v2 = indices[base_idx + 2];

        // 获取/创建线性边中点
        let mid01 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v0, v1);
        let mid12 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v1, v2);
        let mid20 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v2, v0);

        // 标准4分细分：4个小三角形
        // 1. (v0, mid01, mid20)
        new_indices.extend_from_slice(&[v0, mid01, mid20]);

        // 2. (mid01, v1, mid12)
        new_indices.extend_from_slice(&[mid01, v1, mid12]);

        // 3. (mid20, mid12, v2)
        new_indices.extend_from_slice(&[mid20, mid12, v2]);

        // 4. (mid01, mid12, mid20) - 中心三角形
        new_indices.extend_from_slice(&[mid01, mid12, mid20]);
    }
    Ok((new_vertices, new_indices, edge_midpoints))
}

/// 为整个网格计算顶点法向量
///
/// 顶点法向量是共享该顶点的所有三角形面法向量的平均值。
/// 这对于平滑着色和几何操作（如曲面细分）至关重要。
///
/// # 参数
/// * `vertices` - 网格的顶点列表
/// * `indices` - 网格的三角形索引列表
///
/// # 返回值
/// * `Vec<[f32; 3]>` - 每个顶点的归一化法向量列表
fn calculate_vertex_normals(vertices: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut vertex_normals = vec![[0.0, 0.0, 0.0]; vertices.len()];
    let num_triangles = indices.len() / 3;

    for i in 0..num_triangles {
        let i0 = indices[i * 3] as usize;
        let i1 = indices[i * 3 + 1] as usize;
        let i2 = indices[i * 3 + 2] as usize;

        // 检查索引是否在边界内
        if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
            continue; // 跳过无效的三角形
        }

        let v0 = vertices[i0];
        let v1 = vertices[i1];
        let v2 = vertices[i2];

        let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

        let normal = [
            edge1[1] * edge2[2] - edge1[2] * edge2[1],
            edge1[2] * edge2[0] - edge1[0] * edge2[2],
            edge1[0] * edge2[1] - edge1[1] * edge2[0],
        ];

        // 将面法向量累加到每个顶点
        vertex_normals[i0][0] += normal[0];
        vertex_normals[i0][1] += normal[1];
        vertex_normals[i0][2] += normal[2];

        vertex_normals[i1][0] += normal[0];
        vertex_normals[i1][1] += normal[1];
        vertex_normals[i1][2] += normal[2];

        vertex_normals[i2][0] += normal[0];
        vertex_normals[i2][1] += normal[1];
        vertex_normals[i2][2] += normal[2];
    }

    // 归一化所有顶点法向量
    for normal in vertex_normals.iter_mut() {
        let mag = (normal[0].powi(2) + normal[1].powi(2) + normal[2].powi(2)).sqrt();
        if mag > 1e-6 {
            normal[0] /= mag;
            normal[1] /= mag;
            normal[2] /= mag;
        }
    }

    vertex_normals
}

/// 为细分网格插值属性数据
///
/// 此函数处理细分网格时所有顶点和单元格属性的插值。
/// 它处理点属性（需要为新边中点插值）和单元格属性（需要扩展，因为每个原始单元格变成4个新三角形）。
///
/// # 参数
/// * `geometry` - 包含属性的原始几何数据
/// * `edge_midpoint_map` - 从边对到其中点顶点索引的映射
/// * `_original_vertex_count` - 原始网格中的顶点数量（未使用但保留以备将来使用）
/// * `new_vertex_count` - 细分网格中的总顶点数量
///
/// # 返回值
/// * `Ok(HashMap)` - 具有插值值的新属性数据
/// * `Err(VtkError)` - 如果插值失败，返回错误
///
/// # 属性处理
/// - **点属性**：为新边中点顶点插值
/// - **单元格属性**：扩展，使每个原始单元格值复制4次
///
/// # 支持的属性类型
/// - 带查找表的标量数据
/// - 颜色标量数据（RGB/RGBA）
/// - 向量数据（3D向量）
/// - 张量数据（3x3矩阵）
fn interpolate_attributes_for_subdivision(
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
                    // 处理点属性：需要为新边中点插值
                    let interpolated_attr = interpolate_point_attribute_for_subdivision(
                        attr,
                        edge_midpoint_map,
                        new_vertex_count,
                    )?;
                    new_attributes.insert((name.clone(), location.clone()), interpolated_attr);
                }
                AttributeLocation::Cell => {
                    // 单元格属性需要扩展，因为每个原始单元格现在对应多个新三角形
                    let expansion_factor = 4; // 标准细分创建4个三角形
                    let expanded_attr =
                        expand_cell_attribute_for_subdivision(attr, expansion_factor)?;
                    new_attributes.insert((name.clone(), location.clone()), expanded_attr);
                }
            }
        }
    }

    Ok(new_attributes)
}

/// 为细分插值点属性数据
///
/// 此函数处理细分过程中创建新边中点顶点时点（顶点）属性的插值。
/// 每个中点顶点从其两个边端点顶点插值得到属性值。
///
/// # 参数
/// * `attr` - 要插值的原始属性数据
/// * `edge_midpoint_map` - 从边对到其中点顶点索引的映射
/// * `new_vertex_count` - 细分网格中的总顶点数量
///
/// # 返回值
/// * `Ok(AttributeType)` - 具有插值值的新属性数据
/// * `Err(VtkError)` - 如果插值失败，返回错误
///
/// # 插值方法
/// 对所有属性类型使用线性插值（算术平均值）：
/// `插值值 = (值0 + 值1) / 2`
///
/// # 支持的属性类型
/// - **标量**：每个顶点单个值，带可选查找表
/// - **颜色标量**：多分量颜色值（RGB、RGBA等）
/// - **向量**：3D向量数据（速度、法线等）
/// - **张量**：3x3张量矩阵（应力、应变等）
///
/// # 错误处理
/// - 当原始顶点数据缺失时使用默认值
/// - 对所有数组访问进行边界检查
fn interpolate_point_attribute_for_subdivision(
    attr: &AttributeType,
    edge_midpoint_map: &HashMap<(u32, u32), u32>,
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

/// 为细分扩展单元格属性数据
///
/// 由于每个原始三角形在细分过程中变成4个新三角形，
/// 单元格属性需要相应扩展。每个原始单元格的属性值复制4次用于4个新三角形。
///
/// # 参数
/// * `attr` - 要扩展的原始单元格属性数据
/// * `expansion_factor` - 复制原始属性的次数
///
/// # 返回值
/// * `Ok(AttributeType)` - 扩展的属性数据，大小为原始的4倍
/// * `Err(VtkError)` - 如果扩展失败，返回错误
///
/// # 扩展策略
/// 每个原始单元格值简单地复制4次：
/// `[值] -> [值, 值, 值, 值]`
///
/// 这保持了原始属性语义，同时确保每个新三角形具有与其父三角形相同的属性值。
///
/// # 支持的属性类型
/// - **标量**：带查找表的单个值
/// - **颜色标量**：多分量颜色数据
/// - **向量**：3D向量数据
/// - **张量**：3x3张量矩阵
fn expand_cell_attribute_for_subdivision(
    attr: &AttributeType,
    expansion_factor: usize,
) -> Result<AttributeType, VtkError> {
    match attr {
        AttributeType::Scalar {
            num_comp,
            data,
            table_name,
            lookup_table,
        } => {
            let mut new_data = Vec::with_capacity(data.len() * expansion_factor);

            // 每个原始单元格值复制4次
            for &value in data.iter() {
                for _ in 0..expansion_factor {
                    new_data.push(value);
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
            let mut new_data = Vec::with_capacity(data.len() * expansion_factor);

            // 每个原始单元格颜色复制4次
            for color in data.iter() {
                for _ in 0..expansion_factor {
                    new_data.push(color.clone());
                }
            }

            Ok(AttributeType::ColorScalar {
                nvalues: *nvalues,
                data: new_data,
            })
        }
        AttributeType::Vector(data) => {
            let mut new_data = Vec::with_capacity(data.len() * expansion_factor);

            // 每个原始单元格向量复制4次
            for &vector in data.iter() {
                for _ in 0..expansion_factor {
                    new_data.push(vector);
                }
            }

            Ok(AttributeType::Vector(new_data))
        }
        AttributeType::Tensor(data) => {
            let mut new_data = Vec::with_capacity(data.len() * expansion_factor);

            // 每个原始单元格张量复制4次
            for &tensor in data.iter() {
                for _ in 0..expansion_factor {
                    new_data.push(tensor);
                }
            }

            Ok(AttributeType::Tensor(new_data))
        }
    }
}

/// 为细分网格生成三角形到单元格映射
///
/// 细分后，每个原始三角形变成4个新三角形。
/// 此函数创建从新三角形回到其原始父单元格的映射，保持单元格关联。
///
/// # 参数
/// * `original_mapping` - 细分前的原始三角形到单元格映射
///
/// # 返回值
/// * `Vec<usize>` - 长度为原始4倍的新映射，其中每个原始单元格索引复制4次用于4个子三角形
///
/// # 映射策略
/// 对于每个映射到单元格C的原始三角形，4个新三角形都映射到同一个单元格C：
/// ```
/// 原始: [三角形0 -> 单元格0, 三角形1 -> 单元格1, ...]
/// 之后: [tri0_0 -> 单元格0, tri0_1 -> 单元格0, tri0_2 -> 单元格0, tri0_3 -> 单元格0,
///        tri1_0 -> 单元格1, tri1_1 -> 单元格1, tri1_2 -> 单元格1, tri1_3 -> 单元格1, ...]
/// ```
///
/// 这保持了细分操作后网格三角形和数据单元格之间的关系。
fn generate_subdivided_triangle_mapping(original_mapping: &[usize]) -> Vec<usize> {
    let mut new_mapping = Vec::with_capacity(original_mapping.len() * 4);

    for &cell_idx in original_mapping.iter() {
        // 每个原始三角形对应的单元格现在对应4个新三角形
        for _ in 0..4 {
            new_mapping.push(cell_idx);
        }
    }

    new_mapping
}

/// 为细分网格生成默认三角形到单元格映射
///
/// 当原始几何数据没有三角形到单元格映射时，
/// 此函数创建一个默认映射，其中每个原始三角形被视为自己的单元格。
/// 细分后，每组4个子三角形映射回其父三角形索引。
///
/// # 参数
/// * `num_original_triangles` - 原始网格中的三角形数量
///
/// # 返回值
/// * `Vec<usize>` - 默认映射，其中三角形组映射到顺序索引
///
/// # 默认映射策略
/// ```
/// 原始三角形:  [tri0, tri1, tri2, ...]
/// 视为单元格:  [单元格0, 单元格1, 单元格2, ...]
/// 细分后:      [tri0_0->单元格0, tri0_1->单元格0, tri0_2->单元格0, tri0_3->单元格0,
///               tri1_0->单元格1, tri1_1->单元格1, tri1_2->单元格1, tri1_3->单元格1, ...]
/// ```
///
/// 这确保每组4个细分三角形保持与其原始父三角形的关联。
fn generate_default_triangle_mapping(num_original_triangles: usize) -> Vec<usize> {
    let mut mapping = Vec::with_capacity(num_original_triangles * 4);

    for triangle_idx in 0..num_original_triangles {
        // 每个原始三角形对应4个新三角形
        for _ in 0..4 {
            mapping.push(triangle_idx);
        }
    }

    mapping
}

/// 获取或创建边中点顶点
///
/// 此函数实现边中点缓存以避免重复计算。
/// 当边的中点已存在时，它返回缓存的索引。
/// 否则，它创建一个新的中点顶点并缓存它。
///
/// # 参数
/// * `edge_midpoints` - 边中点HashMap缓存
/// * `vertices` - 用于添加新中点的顶点列表的可变引用
/// * `v0` - 边的第一个顶点索引
/// * `v1` - 边的第二个顶点索引
///
/// # 返回值
/// * `u32` - 中点顶点的索引（已存在或新创建的）
///
/// # 边排序
/// 为确保一致性，边以较小的顶点索引优先存储。
/// 这防止了同一边在不同顶点排序下的重复中点。
///
/// # 中点计算
/// 中点计算为两个端点坐标的算术平均值：
/// `中点 = (位置0 + 位置1) / 2`
fn get_or_create_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
) -> u32 {
    // 确保一致的边顶点排序
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // 如果中点已存在，直接返回
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    // 计算中点坐标
    let pos0 = vertices[v0 as usize];
    let pos1 = vertices[v1 as usize];
    let midpoint = [
        (pos0[0] + pos1[0]) * 0.5,
        (pos0[1] + pos1[1]) * 0.5,
        (pos0[2] + pos1[2]) * 0.5,
    ];

    // 添加新顶点
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // 记录边中点映射
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}
