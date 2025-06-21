use vtkio::model::{self, VertexNumbers};

/// 二阶三角形数据结构
///
/// 二阶三角形包含6个控制点：3个角点和3个边中点
/// 顶点布局：
/// - vertices[0,1,2]: 三个角顶点
/// - vertices[3]: 边0-1的中点
/// - vertices[4]: 边1-2的中点  
/// - vertices[5]: 边2-0的中点
///
/// 对于渲染，只使用角顶点 [0,1,2]
/// 边中点 [3,4,5] 保留用于后续的细分操作
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct QuadraticTriangle {
    /// 6个控制点的索引：[v0, v1, v2, m01, m12, m20]
    pub vertices: [u32; 6],
}

#[allow(dead_code)]
impl QuadraticTriangle {
    /// 创建新的二阶三角形
    pub fn new(vertices: [u32; 6]) -> Self {
        Self { vertices }
    }

    /// 获取角顶点索引（用于渲染）
    pub fn corner_vertices(&self) -> [u32; 3] {
        [self.vertices[0], self.vertices[1], self.vertices[2]]
    }

    /// 获取边中点索引（用于细分）
    pub fn edge_midpoints(&self) -> [u32; 3] {
        [self.vertices[3], self.vertices[4], self.vertices[5]]
    }

    /// 获取所有顶点索引
    pub fn all_vertices(&self) -> [u32; 6] {
        self.vertices
    }

    /// 转换为线性三角形（只使用角顶点）
    pub fn to_linear_triangle(&self) -> [u32; 3] {
        self.corner_vertices()
    }
}

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
/// * (triangle index list, triangle to original cell mapping)
pub fn triangulate_cells(cells: model::Cells) -> (Vec<u32>, Vec<usize>) {
    // 1. initialize parameters
    // allocate memory according to triangle initially, if small, it will re-allocate
    let mut indices = Vec::<u32>::with_capacity(cells.num_cells() * 3);
    let mut triangle_to_cell_mapping = Vec::new();

    // 2. handle different data formats directly
    // Legacy format
    match &cells.cell_verts {
        VertexNumbers::Legacy { .. } => {
            let (_, data) = cells.cell_verts.into_legacy();
            let mut data_iter = data.iter().copied().peekable();

            // iterate over all cells
            for (cell_idx, cell_type) in cells.types.iter().enumerate() {
                if data_iter.peek().is_none() {
                    panic!("Cell type list longer than available data");
                }
                // load the number of each cell (first number of each row of cell)
                let num_vertices = data_iter.next().expect("Missing vertex count") as usize;
                // load indices of vertices of this cell
                let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

                // process the cell
                process_cell(
                    &mut indices,
                    &mut triangle_to_cell_mapping,
                    cell_idx,
                    cell_type,
                    &vertices,
                );
            }

            // check whether data all consumed
            if data_iter.next().is_some() {
                panic!(
                    "{} bytes of extra data remaining after processing",
                    data_iter.count() + 1
                );
            }
        }
        // XML format
        VertexNumbers::XML { .. } => {
            let (connectivity, offsets) = cells.cell_verts.into_xml();

            // iterate over all cells using offset array
            let mut start_idx = 0;
            for (cell_idx, cell_type) in cells.types.iter().enumerate() {
                if cell_idx >= offsets.len() {
                    panic!(
                        "Cell index {} exceeds offset array length {}",
                        cell_idx,
                        offsets.len()
                    );
                }

                // get the end index of current cell in connectivity array
                let end_idx = offsets[cell_idx] as usize;

                // check boundary
                if end_idx > connectivity.len() {
                    panic!(
                        "Offset {} exceeds connectivity array length {}",
                        end_idx,
                        connectivity.len()
                    );
                }

                // extract the vertex indices of current cell (convert u64 to u32)
                let vertices: Vec<u32> = connectivity[start_idx..end_idx]
                    .iter()
                    .map(|&x| x as u32)
                    .collect();

                // process the cell
                process_cell(
                    &mut indices,
                    &mut triangle_to_cell_mapping,
                    cell_idx,
                    cell_type,
                    &vertices,
                );

                // update the start index of next cell
                start_idx = end_idx;
            }
        }
    }

    (indices, triangle_to_cell_mapping)
}

/// 处理单个单元格的三角化
fn process_cell(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    cell_idx: usize,
    cell_type: &model::CellType,
    vertices: &[u32],
) {
    // 4. record the length of current index list, for calculating how many triangles this cell generates
    // use it to check whether the mapping is correct
    let initial_index_count = indices.len();

    // 5. process data according to topology
    match cell_type {
        // vertex
        model::CellType::Vertex => {
            // validate vertex count
            if vertices.len() != 1 {
                panic!("Invalid vertex count: {} (expected 1)", vertices.len());
            }
            // convert single vertex to degenerate triangle (use same vertex three times)
            indices.extend_from_slice(&[vertices[0], vertices[0], vertices[0]]);
            // add mapping relation
            triangle_to_cell_mapping.push(cell_idx);
        }
        // line
        model::CellType::Line => {
            if vertices.len() != 2 {
                panic!("Invalid line vertex count: {} (expected 2)", vertices.len());
            }
            // convert line to degenerate triangle (use two same vertices)
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[1]]);
            // add mapping relation
            triangle_to_cell_mapping.push(cell_idx);
        }
        // triangle
        model::CellType::Triangle => {
            if vertices.len() != 3 {
                panic!(
                    "Invalid triangle vertex count: {} (expected 3)",
                    vertices.len()
                );
            }
            // push indices of this cell to indices list
            indices.extend(vertices);
            // one triangle, one mapping
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Quad => {
            // decompose quad into two triangles
            if vertices.len() != 4 {
                panic!("Invalid quad vertex count: {} (expected 4)", vertices.len());
            }
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            // two triangles, two mappings
            triangle_to_cell_mapping.push(cell_idx);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Tetra => {
            // tetrahedron decomposed into 4 triangles
            if vertices.len() != 4 {
                panic!(
                    "Invalid tetrahedron vertex count: {} (expected 4)",
                    vertices.len()
                );
            }
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
            indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
            // 4 triangles, 4 mappings
            for _ in 0..4 {
                triangle_to_cell_mapping.push(cell_idx);
            }
        }

        // quadratic edge
        model::CellType::QuadraticEdge => {
            // QuadraticEdge has 3 vertices: two endpoints and one midpoint
            if vertices.len() != 3 {
                panic!(
                    "Invalid quadratic edge vertex count: {} (expected 3)",
                    vertices.len()
                );
            }
            // decompose quadratic edge into two linear edges, each edge converted to degenerate triangle
            // first segment: from start to midpoint
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[2]]);
            // second segment: from midpoint to end
            indices.extend_from_slice(&[vertices[2], vertices[1], vertices[1]]);
            // add two mapping relations
            triangle_to_cell_mapping.push(cell_idx);
            triangle_to_cell_mapping.push(cell_idx);
        }

        // quadratic triangle - 只使用角点进行渲染
        model::CellType::QuadraticTriangle => {
            // quadratic triangle has 6 vertices: 3 corner vertices + 3 edge midpoints
            if vertices.len() != 6 {
                panic!(
                    "Invalid quadratic triangle vertex count: {} (expected 6)",
                    vertices.len()
                );
            }
            // 只使用角顶点渲染，不使用中点
            // vertex layout:
            // vertices[0,1,2] are corner vertices (used for rendering)
            // vertices[3,4,5] are edge midpoints (ignored for rendering)

            // 只创建一个线性三角形，使用角顶点
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);

            // 一个三角形一个映射
            triangle_to_cell_mapping.push(cell_idx);
        }

        _ => {
            println!("Unsupported cell type: {:?}", cell_type);
            // try to convert other types to triangles, or throw an error
            // here we add a general processing, suitable for simple convex polygons
            if vertices.len() >= 3 {
                // process simple convex polygon using fan triangulation algorithm
                let fan_indices = triangulate_fan(vertices);
                indices.extend(fan_indices);
                // multiple triangles, multiple mappings
                for _ in 0..(vertices.len() - 2) {
                    triangle_to_cell_mapping.push(cell_idx);
                }
            }
        }
    }

    // 6. validate whether the mapping is correct
    let triangles_added = (indices.len() - initial_index_count) / 3;
    let mappings_added = triangle_to_cell_mapping.len() - (initial_index_count / 3);
    if triangles_added != mappings_added {
        println!(
            "Warning: Triangle count ({}) does not match mapping count ({})",
            triangles_added, mappings_added
        );
        // fill the mapping
        while (triangle_to_cell_mapping.len() - (initial_index_count / 3)) < triangles_added {
            triangle_to_cell_mapping.push(cell_idx);
        }
    }
}
