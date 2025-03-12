use vtkio::model;

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
        indices.push(vertices[i]);   // 当前点
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
/// * 三角形索引列表
pub fn triangulate_polygon(topology: model::VertexNumbers) -> Vec<u32> {
    let mut indices = Vec::new();
    let poly_data = topology.into_legacy();

    let num_cells = poly_data.0;
    // 创建迭代器
    let mut data_iter = poly_data.1.iter().copied().peekable();

    // 遍历所有单元格
    for _i in 0..num_cells {
        if data_iter.peek().is_none() {
            println!("警告: 数据迭代器为空，可能未完全解析");
            break;
        }

        // 加载每个单元格的顶点数量（每个多边形的第一个值）
        let num_vertices = match data_iter.next() {
            Some(n) => n as usize,
            None => {
                println!("警告: 缺少顶点数量");
                break;
            }
        };

        // 收集多边形的顶点索引
        let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

        if vertices.len() != num_vertices {
            println!(
                "警告: 获取的顶点数({})少于预期({})",
                vertices.len(),
                num_vertices
            );
        }

        if vertices.len() < 3 {
            // 顶点少于3个，无法形成三角形
            println!("警告: 顶点数量不足，无法形成三角形");
            continue;
        }

        // 根据顶点数量选择合适的三角化方法
        match vertices.len() {
            3 => {
                // 已经是三角形，直接添加
                indices.extend_from_slice(&vertices);
            }
            4 => {
                // 四边形分解为两个三角形
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            }
            _ => {
                // 多于4个顶点的多边形，使用扇形三角化
                indices.extend(triangulate_fan(&vertices));
            }
        }
    }

    // 检查是否有剩余数据
    if data_iter.next().is_some() {
        println!("警告: 处理后仍有额外数据剩余，可能未完全解析");
    }

    indices
}

/// 三角化不同类型的单元格
/// 
/// # 参数
/// * `cells` - 单元格数据
/// 
/// # 返回值
/// * 三角形索引列表
pub fn triangulate_cells(cells: model::Cells) -> Vec<u32> {
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
                if num_vertices != 4 {
                    panic!("Invalid tetrahedron vertex count: {} (expected 4)", num_vertices);
                }
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
                indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
            }

            _ => {
                println!("Unsupported cell type: {:?}", cell_type);
                // 可以尝试将其他类型也转换为三角形，或者抛出错误
                // 这里我们添加了通用处理，适用于简单凸多边形
                if num_vertices >= 3 {
                    // 使用扇形三角化算法处理简单凸多边形
                    indices.extend(triangulate_fan(&vertices));
                }
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

