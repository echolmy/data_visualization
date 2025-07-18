//! # 自适应网格细分模块
//!
//! 本模块为三角网格提供无限细分能力，支持线性三角形（3个顶点）、二次三角形（6个顶点）和二次边（3个顶点）。
//!
//! ## 核心功能
//! - 标准4分细分：每个三角形分成4个小三角形
//! - 二次边细分：每个二次边分成2个子边
//! - 线性插值：边中点使用线性插值计算
//! - 二次插值：使用二次形函数进行精确插值
//! - 属性插值：支持标量、向量、张量等属性的插值
//! - 映射维护：保持三角形到单元格的映射关系
//!
//! ## 二次形函数插值
//!
//! 基于图8-17中定义的二次拉格朗日形函数，本模块实现精确的二次三角形细分：
//!
//! ### 参数坐标系统
//! - p0位于(r=0, s=0) - 角顶点0
//! - p1位于(r=1, s=0) - 角顶点1  
//! - p2位于(r=0, s=1) - 角顶点2
//! - p3位于(r=0.5, s=0) - 边01中点
//! - p4位于(r=0.5, s=0.5) - 边12中点
//! - p5位于(r=0, s=0.5) - 边20中点
//!
//! ### 形函数定义
//! - W0 = (1 - r - s)(2(1 - r - s) - 1) - 对应p0
//! - W1 = r(2r - 1) - 对应p1
//! - W2 = s(2s - 1) - 对应p2
//! - W3 = 4r(1 - r - s) - 对应p3
//! - W4 = 4rs - 对应p4
//! - W5 = 4s(1 - r - s) - 对应p5
//!
//! ### 插值公式
//! 任意参数坐标(r, s)处的坐标为：
//! ```
//! P(r,s) = W0*p0 + W1*p1 + W2*p2 + W3*p3 + W4*p4 + W5*p5
//! ```
//!
//! ## 二次边形函数插值
//!
//! 基于图8-16中定义的二次边拉格朗日形函数，本模块实现精确的二次边细分：
//!
//! ### 参数坐标系统
//! - p0位于r=0处 - 起始端点
//! - p1位于r=1处 - 结束端点
//! - p2位于r=0.5处 - 边中点
//!
//! ### 形函数定义
//! - W0 = 2(r - 0.5)(r - 1) - 对应p0
//! - W1 = 2r(r - 0.5) - 对应p1
//! - W2 = 4r(1 - r) - 对应p2
//!
//! ### 插值公式
//! 任意参数坐标r处的坐标为：
//! ```
//! P(r) = W0*p0 + W1*p1 + W2*p2
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! // 对网格进行一次细分
//! let subdivided_geometry = subdivide_mesh(&geometry)?;
//! ```

use super::{
    AttributeLocation, AttributeType, GeometryData, QuadraticEdge, QuadraticTriangle, VtkError,
};
use bevy::utils::HashMap;

// ============================================================================
// 公共接口
// ============================================================================

/// 对网格进行细分
///
/// 这是主要的细分接口，支持线性和二次三角网格的细分。
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

    // 执行细分操作
    let (
        new_vertices,
        new_indices,
        edge_midpoint_map,
        new_quadratic_triangles,
        new_quadratic_edges,
    ) = match (&geometry.quadratic_triangles, &geometry.quadratic_edges) {
        (Some(quadratic_triangles), quadratic_edges_opt) => {
            println!("网格包含二阶三角形，使用二次形函数插值");
            // 使用二次形函数插值进行细分
            let (vertices, indices, edge_map, quad_triangles) =
                quadratic_4_subdivision(original_vertices, original_indices, quadratic_triangles)?;

            // 如果还有二次边，也进行细分
            let (final_vertices, quad_edges) = if let Some(quadratic_edges) = quadratic_edges_opt {
                println!("同时处理二阶边细分");
                let (edge_vertices, subdivided_edges) =
                    quadratic_edge_2_subdivision(&vertices, quadratic_edges)?;
                (edge_vertices, subdivided_edges)
            } else {
                (vertices, Vec::new())
            };

            (
                final_vertices,
                indices,
                edge_map,
                quad_triangles,
                quad_edges,
            )
        }
        (None, Some(quadratic_edges)) => {
            println!("网格只包含二阶边，使用边形函数插值");
            // 处理二次边细分
            let (edge_vertices, subdivided_edges) =
                quadratic_edge_2_subdivision(original_vertices, quadratic_edges)?;

            // 对于只有边的情况，也执行常规的三角形细分（如果有三角形的话）
            let (vertices, indices, edge_map) =
                smooth_4_subdivision(&edge_vertices, original_indices)?;

            (vertices, indices, edge_map, Vec::new(), subdivided_edges)
        }
        (None, None) => {
            println!("网格不包含二阶元素，使用线性插值");
            // 执行标准4分细分
            let (vertices, indices, edge_map) =
                smooth_4_subdivision(original_vertices, original_indices)?;
            (vertices, indices, edge_map, Vec::new(), Vec::new())
        }
    };

    // 插值属性数据
    let new_attributes = if let Some(attrs) = &geometry.attributes {
        interpolate_attributes_for_subdivision(
            attrs,
            &edge_midpoint_map,
            original_vertices.len(),
            new_vertices.len(),
        )?
    } else {
        HashMap::new()
    };

    // 生成新的三角形到单元格映射
    let new_triangle_to_cell_mapping =
        if let Some(original_mapping) = &geometry.triangle_to_cell_mapping {
            generate_subdivided_triangle_mapping(original_mapping)
        } else {
            // 如果原始几何数据没有映射，生成默认映射
            generate_default_triangle_mapping(num_triangles)
        };

    // 创建新的几何数据
    let mut new_geometry = GeometryData::new(new_vertices, new_indices, new_attributes);
    new_geometry.triangle_to_cell_mapping = Some(new_triangle_to_cell_mapping);

    // 如果有新的二次三角形，添加到几何数据中
    if !new_quadratic_triangles.is_empty() {
        new_geometry = new_geometry.add_quadratic_triangles(new_quadratic_triangles);
        println!(
            "生成了 {} 个二次三角形",
            new_geometry.quadratic_triangles.as_ref().unwrap().len()
        );
    }

    // 如果有新的二次边，添加到几何数据中
    if !new_quadratic_edges.is_empty() {
        new_geometry = new_geometry.add_quadratic_edges(new_quadratic_edges);
        println!(
            "生成了 {} 个二次边",
            new_geometry.quadratic_edges.as_ref().unwrap().len()
        );
    }

    println!(
        "细分完成: {} 个顶点, {} 个三角形",
        new_geometry.vertices.len(),
        new_geometry.indices.len() / 3
    );

    Ok(new_geometry)
}

// ============================================================================
// 核心细分算法
// ============================================================================

/// 二次三角形4分细分 - 使用二次形函数插值
///
/// 实现基于二次形函数的4分细分算法，每个二次三角形分成4个完整的二次三角形。
/// 使用二次拉格朗日插值计算新的边中点和面中点，保持曲面的精确度。
/// 每个子三角形都包含完整的6个控制点，支持进一步的细分。
///
/// # 参数
/// * `vertices` - 原始顶点列表
/// * `indices` - 原始索引列表（只使用角顶点）
/// * `quadratic_triangles` - 二次三角形数据（包含完整的6个控制点）
///
/// # 返回值
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>, Vec<QuadraticTriangle>))` - (新顶点列表, 新索引列表, 边中点映射, 新的二次三角形列表)
/// * `Err(VtkError)` - 如果细分失败，返回错误信息
///
/// # 二次形函数
/// 使用图8-17中定义的6个二次拉格朗日形函数：
/// - W0 = (1 - r - s)(2(1 - r - s) - 1)
/// - W1 = r(2r - 1)
/// - W2 = s(2s - 1)
/// - W3 = 4r(1 - r - s)
/// - W4 = 4rs
/// - W5 = 4s(1 - r - s)
///
/// # 细分策略
/// 每个二次三角形分成4个完整的二次子三角形：
/// 1. 左上角子三角形：(v0, mid01, mid20) + 相应的边中点
/// 2. 右下角子三角形：(mid01, v1, mid12) + 相应的边中点  
/// 3. 左下角子三角形：(mid20, mid12, v2) + 相应的边中点
/// 4. 中心子三角形：(mid01, mid12, mid20) + 相应的边中点
fn quadratic_4_subdivision(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
    quadratic_triangles: &Vec<QuadraticTriangle>,
) -> Result<
    (
        Vec<[f32; 3]>,
        Vec<u32>,
        HashMap<(u32, u32), u32>,
        Vec<QuadraticTriangle>,
    ),
    VtkError,
> {
    let num_triangles = indices.len() / 3;
    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::with_capacity(num_triangles * 4 * 3);
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();
    let mut new_quadratic_triangles = Vec::new();

    println!(
        "二次三角形细分：处理 {} 个二次三角形",
        quadratic_triangles.len()
    );

    for (_triangle_idx, quadratic_tri) in quadratic_triangles.iter().enumerate() {
        // 使用便利方法获取控制点索引，提高代码可读性和类型安全性
        let corner_verts = quadratic_tri.corner_vertices(); // [v0, v1, v2] - 三个角顶点
        let edge_mids = quadratic_tri.edge_midpoints(); // [m01, m12, m20] - 三个边中点

        // 获取角顶点坐标
        let p0 = vertices[corner_verts[0] as usize]; // 角顶点0
        let p1 = vertices[corner_verts[1] as usize]; // 角顶点1
        let p2 = vertices[corner_verts[2] as usize]; // 角顶点2

        // 获取边中点坐标
        let p3 = vertices[edge_mids[0] as usize]; // 边01中点
        let p4 = vertices[edge_mids[1] as usize]; // 边12中点
        let p5 = vertices[edge_mids[2] as usize]; // 边20中点

        // 确保按照逆时针顺序处理二次三角形
        // 原始二次三角形控制点顺序: [v0, v1, v2, m01, m12, m20]

        // 使用二次形函数计算新的边中点
        // 主边中点：3个主要边的中点
        let mid01 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[0],
            corner_verts[1],
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.0), // 边01中点的参数坐标
        );

        let mid12 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[1],
            corner_verts[2],
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.5), // 边12中点的参数坐标
        );

        let mid20 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[2],
            corner_verts[0],
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.5), // 边20中点的参数坐标
        );

        // 计算新边的中点（子三角形之间的边）
        // mid01 到 v0 的边中点
        let mid_mid01_v0 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[0],
            mid01,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.0), // 细分后的边中点
        );

        // mid01 到 v1 的边中点
        let mid_mid01_v1 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid01,
            corner_verts[1],
            &[p0, p1, p2, p3, p4, p5],
            (0.75, 0.0), // 细分后的边中点
        );

        // mid12 到 v1 的边中点
        let mid_mid12_v1 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[1],
            mid12,
            &[p0, p1, p2, p3, p4, p5],
            (0.75, 0.25), // 细分后的边中点
        );

        // mid12 到 v2 的边中点
        let mid_mid12_v2 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid12,
            corner_verts[2],
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.75), // 细分后的边中点
        );

        // mid20 到 v2 的边中点
        let mid_mid20_v2 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[2],
            mid20,
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.75), // 细分后的边中点
        );

        // mid20 到 v0 的边中点
        let mid_mid20_v0 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid20,
            corner_verts[0],
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.25), // 细分后的边中点
        );

        // 内部连接边的中点
        let mid_mid01_mid12 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid01,
            mid12,
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.25), // 内部边中点
        );

        let mid_mid12_mid20 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid12,
            mid20,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.5), // 内部边中点
        );

        let mid_mid20_mid01 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid20,
            mid01,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.25), // 内部边中点
        );

        // 生成4个子三角形的索引（逆时针顺序）
        // 1. 左上角子三角形：(v0, mid01, mid20)
        new_indices.extend_from_slice(&[corner_verts[0], mid01, mid20]);
        let quad_tri_1 = QuadraticTriangle::new([
            corner_verts[0],
            mid01,
            mid20, // 角顶点
            mid_mid01_v0,
            mid_mid20_mid01,
            mid_mid20_v0, // 边中点
        ]);
        new_quadratic_triangles.push(quad_tri_1);

        // 2. 右下角子三角形：(mid01, v1, mid12)
        new_indices.extend_from_slice(&[mid01, corner_verts[1], mid12]);
        let quad_tri_2 = QuadraticTriangle::new([
            mid01,
            corner_verts[1],
            mid12, // 角顶点
            mid_mid01_v1,
            mid_mid12_v1,
            mid_mid01_mid12, // 边中点
        ]);
        new_quadratic_triangles.push(quad_tri_2);

        // 3. 左下角子三角形：(mid20, mid12, v2)
        new_indices.extend_from_slice(&[mid20, mid12, corner_verts[2]]);
        let quad_tri_3 = QuadraticTriangle::new([
            mid20,
            mid12,
            corner_verts[2], // 角顶点
            mid_mid12_mid20,
            mid_mid12_v2,
            mid_mid20_v2, // 边中点
        ]);
        new_quadratic_triangles.push(quad_tri_3);

        // 4. 中心子三角形：(mid01, mid12, mid20) - 确保逆时针顺序
        new_indices.extend_from_slice(&[mid01, mid12, mid20]);
        let quad_tri_4 = QuadraticTriangle::new([
            mid01,
            mid12,
            mid20, // 角顶点
            mid_mid01_mid12,
            mid_mid12_mid20,
            mid_mid20_mid01, // 边中点
        ]);
        new_quadratic_triangles.push(quad_tri_4);

        // 生成了4个子二次三角形，按逆时针顺序排列
    }

    println!(
        "总共生成了 {} 个新的二次三角形",
        new_quadratic_triangles.len()
    );

    Ok((
        new_vertices,
        new_indices,
        edge_midpoints,
        new_quadratic_triangles,
    ))
}

/// 使用二次形函数获取或创建边中点顶点
///
/// 此函数使用二次拉格朗日插值计算边中点，而不是简单的线性插值。
/// 它根据参数坐标中点位置 (r=0.5, s=0 或其他组合) 计算精确的曲面位置。
///
/// # 参数
/// * `edge_midpoints` - 边中点HashMap缓存
/// * `vertices` - 用于添加新中点的顶点列表的可变引用
/// * `v0` - 边的第一个顶点索引
/// * `v1` - 边的第二个顶点索引
/// * `control_points` - 二次三角形的6个控制点坐标
/// * `parametric_coords` - 边中点的参数坐标 (r, s)
///
/// # 返回值
/// * `u32` - 中点顶点的索引（已存在或新创建的）
///
/// # 二次插值策略
/// 根据边的类型，使用相应的参数坐标计算中点：
/// - 边01: (r=0.5, s=0)
/// - 边12: (r=0.5, s=0.5)
/// - 边20: (r=0, s=0.5)
fn get_or_create_quadratic_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
    control_points: &[[f32; 3]; 6],
    parametric_coords: (f32, f32),
) -> u32 {
    // 确保一致的边顶点排序
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // 如果中点已存在，直接返回
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    // 使用提供的参数坐标
    let (r, s) = parametric_coords;

    // 使用二次形函数计算中点坐标
    let midpoint = quadratic_interpolation(r, s, control_points);

    // 添加新顶点
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // 记录边中点映射
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}

/// 二次拉格朗日插值函数
///
/// 根据参数坐标 (r, s) 和6个控制点，使用二次形函数计算插值点坐标。
///
/// # 参数
/// * `r` - 参数坐标r
/// * `s` - 参数坐标s
/// * `control_points` - 6个控制点坐标 [p0, p1, p2, p3, p4, p5]
///
/// # 返回值
/// * `[f32; 3]` - 插值得到的点坐标
///
/// # 形函数定义（基于图8-17）
/// - W0 = (1 - r - s)(2(1 - r - s) - 1) - 对应p0（角顶点）
/// - W1 = r(2r - 1) - 对应p1（角顶点）
/// - W2 = s(2s - 1) - 对应p2（角顶点）
/// - W3 = 4r(1 - r - s) - 对应p3（边01中点）
/// - W4 = 4rs - 对应p4（边12中点）
/// - W5 = 4s(1 - r - s) - 对应p5（边20中点）
fn quadratic_interpolation(r: f32, s: f32, control_points: &[[f32; 3]; 6]) -> [f32; 3] {
    let t = 1.0 - r - s; // t = 1 - r - s

    // 计算6个二次形函数值
    let w0 = t * (2.0 * t - 1.0); // W0 = (1-r-s)(2(1-r-s)-1)
    let w1 = r * (2.0 * r - 1.0); // W1 = r(2r-1)
    let w2 = s * (2.0 * s - 1.0); // W2 = s(2s-1)
    let w3 = 4.0 * r * t; // W3 = 4r(1-r-s)
    let w4 = 4.0 * r * s; // W4 = 4rs
    let w5 = 4.0 * s * t; // W5 = 4s(1-r-s)

    // 线性组合计算插值点坐标
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] = w0 * control_points[0][i]  // p0贡献
                  + w1 * control_points[1][i]  // p1贡献
                  + w2 * control_points[2][i]  // p2贡献
                  + w3 * control_points[3][i]  // p3贡献（边01中点）
                  + w4 * control_points[4][i]  // p4贡献（边12中点）
                  + w5 * control_points[5][i]; // p5贡献（边20中点）
    }

    result
}

/// 二次边2分细分 - 使用二次边形函数插值
///
/// 实现基于二次边形函数的2分细分算法，每个二次边分成2个完整的二次子边。
/// 使用二次拉格朗日插值计算新的边点，保持曲线的精确度。
/// 每个子边都包含完整的3个控制点，支持进一步的细分。
///
/// # 参数
/// * `vertices` - 原始顶点列表
/// * `quadratic_edges` - 二次边数据（包含完整的3个控制点）
///
/// # 返回值
/// * `Ok((Vec<[f32; 3]>, Vec<QuadraticEdge>))` - (新顶点列表, 新的二次边列表)
/// * `Err(VtkError)` - 如果细分失败，返回错误信息
///
/// # 二次边形函数
/// 使用图8-16中定义的3个二次拉格朗日形函数：
/// - W0 = 2(r - 0.5)(r - 1)
/// - W1 = 2r(r - 0.5)
/// - W2 = 4r(1 - r)
///
/// # 细分策略
/// 每个二次边分成2个完整的二次子边：
/// 1. 左半段子边：(p0, mid, new_left_mid)
/// 2. 右半段子边：(mid, p1, new_right_mid)
fn quadratic_edge_2_subdivision(
    vertices: &Vec<[f32; 3]>,
    quadratic_edges: &Vec<QuadraticEdge>,
) -> Result<(Vec<[f32; 3]>, Vec<QuadraticEdge>), VtkError> {
    let mut new_vertices = vertices.clone();
    let mut new_quadratic_edges = Vec::new();

    println!("二次边细分：处理 {} 个二次边", quadratic_edges.len());

    for quadratic_edge in quadratic_edges.iter() {
        // 获取二次边的控制点
        let endpoints = quadratic_edge.endpoints();
        let midpoint_idx = quadratic_edge.midpoint();
        let p0 = vertices[endpoints[0] as usize]; // r=0端点
        let p1 = vertices[endpoints[1] as usize]; // r=1端点
        let p2 = vertices[midpoint_idx as usize]; // r=0.5中点

        // 使用二次边形函数计算新的分割点
        // 计算r=0.25处的点（左半段的中点）
        let left_mid = quadratic_edge_interpolation(0.25, &[p0, p1, p2]);
        let left_mid_idx = new_vertices.len() as u32;
        new_vertices.push(left_mid);

        // 计算r=0.75处的点（右半段的中点）
        let right_mid = quadratic_edge_interpolation(0.75, &[p0, p1, p2]);
        let right_mid_idx = new_vertices.len() as u32;
        new_vertices.push(right_mid);

        // 生成2个子边
        // 1. 左半段子边：(p0, 原中点p2, 新left_mid)
        let left_edge = QuadraticEdge::new([
            endpoints[0], // p0
            midpoint_idx, // 原中点，现在是左半段的终点
            left_mid_idx, // 新的左半段中点
        ]);
        new_quadratic_edges.push(left_edge);

        // 2. 右半段子边：(原中点p2, p1, 新right_mid)
        let right_edge = QuadraticEdge::new([
            midpoint_idx,  // 原中点，现在是右半段的起点
            endpoints[1],  // p1
            right_mid_idx, // 新的右半段中点
        ]);
        new_quadratic_edges.push(right_edge);
    }

    println!("总共生成了 {} 个新的二次边", new_quadratic_edges.len());

    Ok((new_vertices, new_quadratic_edges))
}

/// 二次边拉格朗日插值函数
///
/// 根据参数坐标 r 和3个控制点，使用二次边形函数计算插值点坐标。
///
/// # 参数
/// * `r` - 参数坐标r (0 <= r <= 1)
/// * `control_points` - 3个控制点坐标 [p0, p1, p2]
///
/// # 返回值
/// * `[f32; 3]` - 插值得到的点坐标
///
/// # 形函数定义（基于图8-16）
/// - W0 = 2(r - 0.5)(r - 1) - 对应p0（r=0端点）
/// - W1 = 2r(r - 0.5) - 对应p1（r=1端点）
/// - W2 = 4r(1 - r) - 对应p2（r=0.5中点）
fn quadratic_edge_interpolation(r: f32, control_points: &[[f32; 3]; 3]) -> [f32; 3] {
    // 计算3个二次边形函数值
    let w0 = 2.0 * (r - 0.5) * (r - 1.0); // W0 = 2(r-0.5)(r-1)
    let w1 = 2.0 * r * (r - 0.5); // W1 = 2r(r-0.5)
    let w2 = 4.0 * r * (1.0 - r); // W2 = 4r(1-r)

    // 线性组合计算插值点坐标
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] = w0 * control_points[0][i]  // p0贡献
                  + w1 * control_points[1][i]  // p1贡献
                  + w2 * control_points[2][i]; // p2贡献（中点）
    }

    result
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

// ============================================================================
// 属性处理
// ============================================================================

/// 为细分网格插值属性数据
///
/// 此函数处理细分网格时所有顶点和单元格属性的插值。
/// 它处理点属性（需要为新边中点插值）和单元格属性（需要扩展，因为每个原始单元格变成4个新三角形）。
///
/// # 参数
/// * `attributes` - 包含属性的HashMap
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
    attributes: &HashMap<(String, AttributeLocation), AttributeType>,
    edge_midpoint_map: &HashMap<(u32, u32), u32>,
    _original_vertex_count: usize,
    new_vertex_count: usize,
) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
    let mut new_attributes = HashMap::new();

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
                let expanded_attr = expand_cell_attribute_for_subdivision(attr, expansion_factor)?;
                new_attributes.insert((name.clone(), location.clone()), expanded_attr);
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

            // 检测原始数据范围
            let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let range = max_val - min_val;

            if range < 1e-10 {
                // 当原始数据范围极小时（如理论解为常数的情况），使用恒定值
                let avg_val = (min_val + max_val) * 0.5;
                println!(
                    "Original scalar data range is very small ({}), using constant value {} for subdivision",
                    range, avg_val
                );

                // 为所有新边中点设置相同的值，保持"恒定"特性
                for (_, &midpoint_idx) in edge_midpoint_map.iter() {
                    if (midpoint_idx as usize) < new_data.len() {
                        new_data[midpoint_idx as usize] = avg_val;
                    }
                }
            } else {
                // 正常插值处理
                for ((v0, v1), &midpoint_idx) in edge_midpoint_map.iter() {
                    let val0 = data.get(*v0 as usize).copied().unwrap_or(0.0);
                    let val1 = data.get(*v1 as usize).copied().unwrap_or(0.0);
                    let interpolated_val = (val0 + val1) * 0.5;

                    if (midpoint_idx as usize) < new_data.len() {
                        new_data[midpoint_idx as usize] = interpolated_val;
                    }
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

// ============================================================================
// 映射处理
// ============================================================================

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
