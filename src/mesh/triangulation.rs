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
    let cell_data = cells.cell_verts.into_legacy();

    // 2. create iterator, use peekable to check edge condition
    let mut data_iter = cell_data.1.iter().copied().peekable();

    // 3. iterate over all cells
    for (cell_idx, cell_type) in cells.types.iter().enumerate() {
        if data_iter.peek().is_none() {
            panic!("Cell type list longer than available data");
        }
        // load the number of each cell (first number of each row of cell)
        let num_vertices = data_iter.next().expect("Missing vertex count") as usize;
        // load indices of vertices of this cell
        let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

        // 4. record the length of current index list, for calculating how many triangles this cell generates
        // use it to check whether the mapping is correct
        let initial_index_count = indices.len();

        // 5. process data according to topology
        match cell_type {
            // vertex
            model::CellType::Vertex => {
                // validate vertex count
                if num_vertices != 1 {
                    panic!("Invalid vertex count: {} (expected 1)", num_vertices);
                }
                // convert single vertex to degenerate triangle (use same vertex three times)
                indices.extend_from_slice(&[vertices[0], vertices[0], vertices[0]]);
                // add mapping relation
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
            // line
            model::CellType::Line => {
                if num_vertices != 2 {
                    panic!("Invalid line vertex count: {} (expected 2)", num_vertices);
                }
                // convert line to degenerate triangle (use two same vertices)
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[1]]);
                // add mapping relation
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
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
                // one triangle, one mapping
                triangle_to_cell_mapping.push(cell_idx as usize);
            }

            model::CellType::Quad => {
                // decompose quad into two triangles
                if num_vertices != 4 {
                    panic!("Invalid quad vertex count: {} (expected 4)", num_vertices);
                }
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                // two triangles, two mappings
                triangle_to_cell_mapping.push(cell_idx as usize);
                triangle_to_cell_mapping.push(cell_idx as usize);
            }

            model::CellType::Tetra => {
                // tetrahedron decomposed into 4 triangles
                if num_vertices != 4 {
                    panic!(
                        "Invalid tetrahedron vertex count: {} (expected 4)",
                        num_vertices
                    );
                }
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
                indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
                // 4 triangles, 4 mappings
                for _ in 0..4 {
                    triangle_to_cell_mapping.push(cell_idx as usize);
                }
            }

            model::CellType::QuadraticEdge => {
                // 二次边有3个顶点：两个端点和一个中点
                if num_vertices != 3 {
                    panic!(
                        "Invalid quadratic edge vertex count: {} (expected 3)",
                        num_vertices
                    );
                }
                // 将二次边分解为两个线性边，每个边转换为退化三角形
                // 第一段：从起点到中点
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[2]]);
                // 第二段：从中点到终点
                indices.extend_from_slice(&[vertices[2], vertices[1], vertices[1]]);
                // 添加两个映射关系
                triangle_to_cell_mapping.push(cell_idx as usize);
                triangle_to_cell_mapping.push(cell_idx as usize);
            }

            model::CellType::QuadraticTriangle => {
                // 二次三角形有6个顶点：3个角顶点 + 3个边中点
                if num_vertices != 6 {
                    panic!(
                        "Invalid quadratic triangle vertex count: {} (expected 6)",
                        num_vertices
                    );
                }
                // 顶点布局：vertices[0,1,2] 是角顶点，vertices[3,4,5] 是边中点
                // 边中点3在边0-1之间，边中点4在边1-2之间，边中点5在边2-0之间

                // 分解为4个线性三角形：
                // 中心三角形：由3个边中点组成
                indices.extend_from_slice(&[vertices[3], vertices[4], vertices[5]]);
                // 角三角形1：角顶点0及其相邻的两个边中点
                indices.extend_from_slice(&[vertices[0], vertices[3], vertices[5]]);
                // 角三角形2：角顶点1及其相邻的两个边中点
                indices.extend_from_slice(&[vertices[1], vertices[4], vertices[3]]);
                // 角三角形3：角顶点2及其相邻的两个边中点
                indices.extend_from_slice(&[vertices[2], vertices[5], vertices[4]]);

                // 添加4个映射关系
                for _ in 0..4 {
                    triangle_to_cell_mapping.push(cell_idx as usize);
                }
            }

            _ => {
                println!("Unsupported cell type: {:?}", cell_type);
                // try to convert other types to triangles, or throw an error
                // here we add a general processing, suitable for simple convex polygons
                if num_vertices >= 3 {
                    // process simple convex polygon using fan triangulation algorithm
                    let fan_indices = triangulate_fan(&vertices);
                    indices.extend(fan_indices);
                    // multiple triangles, multiple mappings
                    for _ in 0..(vertices.len() - 2) {
                        triangle_to_cell_mapping.push(cell_idx as usize);
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
                triangle_to_cell_mapping.push(cell_idx as usize);
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

    (indices, triangle_to_cell_mapping)
}
