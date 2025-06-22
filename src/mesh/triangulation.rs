use super::QuadraticTriangle;
use vtkio::model::{self, VertexNumbers};

/// 通用三角化模块，提供各种几何体的三角化功能

/// 扇形三角化算法
///
/// 将一个多边形顶点列表分解为三角形，使用第一个顶点作为扇形中心
///
/// # 参数
/// * `vertices` - 多边形顶点索引列表
///
/// # 返回值
/// * 三角形索引列表（每三个为一组）
pub fn triangulate_fan(vertices: &[u32]) -> Vec<u32> {
    // 如果顶点少于3个，无法形成三角形
    if vertices.len() < 3 {
        return Vec::new();
    }

    // 如果已经是三角形，直接返回
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

/// 多边形三角化函数
///
/// 将一个多边形（可能是复杂形状）转换为三角形列表
///
/// # 参数
/// * `topology` - 顶点拓扑结构
///
/// # 返回值
/// * (三角形索引列表, 三角形到原始单元格的映射)
pub fn triangulate_polygon(topology: model::VertexNumbers) -> (Vec<u32>, Vec<usize>) {
    let mut indices = Vec::new();
    let mut triangle_to_cell_mapping = Vec::new();
    let poly_data = topology.into_legacy();

    let num_cells = poly_data.0;
    // 创建迭代器
    let mut data_iter = poly_data.1.iter().copied().peekable();

    // 遍历所有单元格
    for cell_idx in 0..num_cells {
        if data_iter.peek().is_none() {
            println!("Warning: Data iterator is empty, possibly not fully parsed");
            break;
        }

        // 加载每个单元格的顶点数量（每个多边形的第一个值）
        let num_vertices = match data_iter.next() {
            Some(n) => n as usize,
            None => {
                println!("Warning: Missing vertex count");
                break;
            }
        };

        // 收集多边形的顶点索引
        let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

        if vertices.len() != num_vertices {
            println!(
                "Warning: Vertex count ({}) less than expected ({})",
                vertices.len(),
                num_vertices
            );
        }

        if vertices.len() < 3 {
            // 顶点少于3个，无法形成三角形
            println!("Warning: Insufficient vertex count, cannot form triangles");
            continue;
        }

        // 记录当前索引列表的长度，用于计算这个单元格生成了多少个三角形
        let initial_index_count = indices.len();

        // 根据顶点数量选择合适的三角化方法
        match vertices.len() {
            3 => {
                // 已经是三角形，直接添加
                indices.extend_from_slice(&vertices);
                // 一个三角形一个映射
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
            4 => {
                // 四边形分解为两个三角形
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                // 两个三角形两个映射
                triangle_to_cell_mapping.push(cell_idx as usize);
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
            _ => {
                // 多于4个顶点的多边形，使用扇形三角化
                let fan_indices = triangulate_fan(&vertices);
                indices.extend(fan_indices);
                // 多个三角形多个映射
                for _ in 0..(vertices.len() - 2) {
                    triangle_to_cell_mapping.push(cell_idx as usize);
                }
            }
        }

        // 验证是否正确添加了映射
        let triangles_added = (indices.len() - initial_index_count) / 3;
        let mappings_added = triangle_to_cell_mapping.len() - (initial_index_count / 3);
        if triangles_added != mappings_added {
            println!(
                "Warning: Triangle count ({}) does not match mapping count ({})",
                triangles_added, mappings_added
            );
            // 补齐映射
            while (triangle_to_cell_mapping.len() - (initial_index_count / 3)) < triangles_added {
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
        }
    }

    // 检查是否有剩余数据
    if data_iter.next().is_some() {
        println!("Warning: There is still extra data remaining after processing, possibly not fully parsed");
    }

    (indices, triangle_to_cell_mapping)
}

/// triangulate different types of cells, used for UnstructuredGrid type
///
/// # parameters
/// * `cells` - cell data
///
/// # return value
/// * (triangle index list, triangle to original cell mapping, quadratic triangles)
pub fn triangulate_cells(cells: model::Cells) -> (Vec<u32>, Vec<usize>, Vec<QuadraticTriangle>) {
    // 初始化参数
    let mut indices = Vec::<u32>::with_capacity(cells.num_cells() * 3);
    let mut triangle_to_cell_mapping = Vec::new();
    let mut quadratic_triangles = Vec::new();

    // 将所有格式的数据统一为 (cell_type, vertices) 的格式
    let cell_data = extract_cell_data(cells);

    // 处理每个单元格
    for (cell_idx, (cell_type, vertices)) in cell_data.into_iter().enumerate() {
        process_cell(
            &mut indices,
            &mut triangle_to_cell_mapping,
            &mut quadratic_triangles,
            cell_idx,
            &cell_type,
            &vertices,
        );
    }

    (indices, triangle_to_cell_mapping, quadratic_triangles)
}

/// 从cells数据中提取统一格式的单元格数据
///
/// 将Legacy和XML两种格式统一为 (cell_type, vertices) 的列表
fn extract_cell_data(cells: model::Cells) -> Vec<(model::CellType, Vec<u32>)> {
    let mut cell_data = Vec::new();

    match cells.cell_verts {
        VertexNumbers::Legacy { .. } => {
            let data = cells.cell_verts.into_legacy();
            let num_cells = data.0;
            let mut data_iter = data.1.iter().copied().peekable();

            for (cell_idx, cell_type) in cells.types.iter().enumerate() {
                if cell_idx >= num_cells as usize || data_iter.peek().is_none() {
                    break;
                }

                // 获取顶点数量
                let num_vertices = match data_iter.next() {
                    Some(n) => n as usize,
                    None => break,
                };

                // 收集顶点索引
                let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

                if vertices.len() == num_vertices {
                    cell_data.push((cell_type.clone(), vertices));
                }
            }
        }
        VertexNumbers::XML { .. } => {
            let (connectivity, offsets) = cells.cell_verts.into_xml();
            let mut start_idx = 0;

            for (cell_idx, cell_type) in cells.types.iter().enumerate() {
                if cell_idx >= offsets.len() {
                    break;
                }

                let end_idx = offsets[cell_idx] as usize;
                if end_idx > connectivity.len() {
                    break;
                }

                // 提取顶点索引（转换u64到u32）
                let vertices: Vec<u32> = connectivity[start_idx..end_idx]
                    .iter()
                    .map(|&x| x as u32)
                    .collect();

                cell_data.push((cell_type.clone(), vertices));
                start_idx = end_idx;
            }
        }
    }

    cell_data
}

/// 统一的单元格处理函数
///
/// 根据单元格类型进行相应的三角化处理
fn process_cell(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    quadratic_triangles: &mut Vec<QuadraticTriangle>,
    cell_idx: usize,
    cell_type: &model::CellType,
    vertices: &[u32],
) {
    let initial_index_count = indices.len();

    match cell_type {
        // 基本单元格类型
        model::CellType::Vertex => {
            validate_vertex_count(vertices, 1, "vertex");
            // 转换单个顶点为退化三角形（使用同一顶点三次）
            indices.extend_from_slice(&[vertices[0], vertices[0], vertices[0]]);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Line => {
            validate_vertex_count(vertices, 2, "line");
            // 转换线段为退化三角形（使用两个相同顶点）
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[1]]);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Triangle => {
            validate_vertex_count(vertices, 3, "triangle");
            // 直接添加三角形索引
            indices.extend(vertices);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Quad => {
            validate_vertex_count(vertices, 4, "quad");
            // 将四边形分解为两个三角形
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            triangle_to_cell_mapping.push(cell_idx);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Tetra => {
            validate_vertex_count(vertices, 4, "tetrahedron");
            // 四面体分解为4个三角形
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
            indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
            for _ in 0..4 {
                triangle_to_cell_mapping.push(cell_idx);
            }
        }

        // 二阶单元格类型 - 需要特殊处理
        model::CellType::QuadraticEdge => {
            process_quadratic_edge(indices, triangle_to_cell_mapping, cell_idx, vertices);
        }

        model::CellType::QuadraticTriangle => {
            process_quadratic_triangle(
                indices,
                triangle_to_cell_mapping,
                quadratic_triangles,
                cell_idx,
                vertices,
            );
        }

        _ => {
            println!("Unsupported cell type: {:?}", cell_type);
            // 尝试使用扇形三角化处理其他类型
            if vertices.len() >= 3 {
                let fan_indices = triangulate_fan(vertices);
                indices.extend(fan_indices);
                for _ in 0..(vertices.len() - 2) {
                    triangle_to_cell_mapping.push(cell_idx);
                }
            }
        }
    }

    // 验证映射是否正确
    validate_mapping(
        indices,
        triangle_to_cell_mapping,
        initial_index_count,
        cell_idx,
    );
}

/// 处理二阶边
fn process_quadratic_edge(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    cell_idx: usize,
    vertices: &[u32],
) {
    validate_vertex_count(vertices, 3, "quadratic edge");
    // 二阶边分解为two个线性边，每个边转换为退化三角形
    // 第一段：从起点到中点
    indices.extend_from_slice(&[vertices[0], vertices[2], vertices[2]]);
    // 第二段：从中点到终点
    indices.extend_from_slice(&[vertices[2], vertices[1], vertices[1]]);
    triangle_to_cell_mapping.push(cell_idx);
    triangle_to_cell_mapping.push(cell_idx);
}

/// 处理二阶三角形
fn process_quadratic_triangle(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    quadratic_triangles: &mut Vec<QuadraticTriangle>,
    cell_idx: usize,
    vertices: &[u32],
) {
    validate_vertex_count(vertices, 6, "quadratic triangle");

    // 只使用角顶点进行渲染
    // 顶点布局：vertices[0,1,2]是角顶点（用于渲染），vertices[3,4,5]是边中点（用于细分）
    indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
    triangle_to_cell_mapping.push(cell_idx);

    // 创建二阶三角形数据结构（保存完整的6个控制点信息）
    let quadratic_triangle = QuadraticTriangle::new([
        vertices[0],
        vertices[1],
        vertices[2], // 角顶点
        vertices[3],
        vertices[4],
        vertices[5], // 边中点
    ]);

    // 存储二阶三角形供后续细分使用
    quadratic_triangles.push(quadratic_triangle);
}

/// 验证顶点数量
fn validate_vertex_count(vertices: &[u32], expected: usize, cell_type_name: &str) {
    if vertices.len() != expected {
        panic!(
            "Invalid {} vertex count: {} (expected {})",
            cell_type_name,
            vertices.len(),
            expected
        );
    }
}

/// 验证映射关系是否正确
fn validate_mapping(
    indices: &[u32],
    triangle_to_cell_mapping: &mut Vec<usize>,
    initial_index_count: usize,
    cell_idx: usize,
) {
    let triangles_added = (indices.len() - initial_index_count) / 3;
    let mappings_added = triangle_to_cell_mapping.len() - (initial_index_count / 3);

    if triangles_added != mappings_added {
        println!(
            "Warning: Triangle count ({}) does not match mapping count ({})",
            triangles_added, mappings_added
        );
        // 补齐映射
        while (triangle_to_cell_mapping.len() - (initial_index_count / 3)) < triangles_added {
            triangle_to_cell_mapping.push(cell_idx);
        }
    }
}
