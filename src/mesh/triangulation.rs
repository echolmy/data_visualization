use super::{QuadraticEdge, QuadraticTriangle};
use vtkio::model::{self, VertexNumbers};

/// Triangulation module, providing triangulation functionality for various geometric shapes

/// Fan triangulation algorithm
///
/// Decomposes a polygon vertex list into triangles, using the first vertex as the fan center
///
/// # Parameters
/// * `vertices` - Polygon vertex index list
///
/// # Return value
/// * Triangle index list (every three indices form a group)
pub fn triangulate_fan(vertices: &[u32]) -> Vec<u32> {
    // If there are less than 3 vertices, cannot form a triangle
    if vertices.len() < 3 {
        return Vec::new();
    }

    // If it's already a triangle, return directly
    if vertices.len() == 3 {
        return vertices.to_vec();
    }

    // For a polygon with n vertices, need (n-2)*3 indices to store triangles
    let mut indices = Vec::with_capacity((vertices.len() - 2) * 3);

    // Use the first vertex as the center point of the fan
    let center_vertex = vertices[0];

    // Create triangle fan
    for i in 1..vertices.len() - 1 {
        indices.push(center_vertex); // Center point
        indices.push(vertices[i]); // Current point
        indices.push(vertices[i + 1]); // Next point
    }

    indices
}

/// Polygon triangulation function
///
/// Converts a polygon into a list of triangles
///
/// # Parameters
/// * `topology` - Vertex topology structure
///
/// # Return value
/// * (Triangle index list, triangle to original cell mapping)
pub fn triangulate_polygon(topology: model::VertexNumbers) -> (Vec<u32>, Vec<usize>) {
    let mut indices = Vec::new();
    let mut triangle_to_cell_mapping = Vec::new();
    let poly_data = topology.into_legacy();

    let num_cells = poly_data.0;
    // Create iterator
    let mut data_iter = poly_data.1.iter().copied().peekable();

    // Traverse all cells
    for cell_idx in 0..num_cells {
        if data_iter.peek().is_none() {
            println!("Warning: Data iterator is empty, possibly not fully parsed");
            break;
        }

        // Load vertex count for each cell (first value of each polygon)
        let num_vertices = match data_iter.next() {
            Some(n) => n as usize,
            None => {
                println!("Warning: Missing vertex count");
                break;
            }
        };

        // Collect polygon vertex indices
        let vertices: Vec<u32> = data_iter.by_ref().take(num_vertices).collect();

        if vertices.len() != num_vertices {
            println!(
                "Warning: Vertex count ({}) less than expected ({})",
                vertices.len(),
                num_vertices
            );
        }

        if vertices.len() < 3 {
            // Less than 3 vertices, cannot form triangles
            println!("Warning: Insufficient vertex count, cannot form triangles");
            continue;
        }

        // Current index list length to calculate how many triangles this cell generates
        let initial_index_count = indices.len();

        // Choose appropriate triangulation method based on vertex count
        match vertices.len() {
            3 => {
                // Already a triangle, add directly
                indices.extend_from_slice(&vertices);
                // One triangle one mapping
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
            4 => {
                // Decompose quadrilateral into two triangles
                indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
                indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
                // Two triangles two mappings
                triangle_to_cell_mapping.push(cell_idx as usize);
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
            _ => {
                // Polygons with more than 4 vertices, use fan triangulation
                let fan_indices = triangulate_fan(&vertices);
                indices.extend(fan_indices);
                // Multiple triangles multiple mappings
                for _ in 0..(vertices.len() - 2) {
                    triangle_to_cell_mapping.push(cell_idx as usize);
                }
            }
        }

        // Verify if mappings were added correctly
        let triangles_added = (indices.len() - initial_index_count) / 3;
        let mappings_added = triangle_to_cell_mapping.len() - (initial_index_count / 3);
        if triangles_added != mappings_added {
            println!(
                "Warning: Triangle count ({}) does not match mapping count ({})",
                triangles_added, mappings_added
            );
            // Fill missing mappings
            while (triangle_to_cell_mapping.len() - (initial_index_count / 3)) < triangles_added {
                triangle_to_cell_mapping.push(cell_idx as usize);
            }
        }
    }

    // Check if there's remaining data
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
/// * (triangle index list, triangle to original cell mapping, quadratic triangles, quadratic edges)
pub fn triangulate_cells(
    cells: model::Cells,
) -> (
    Vec<u32>,
    Vec<usize>,
    Vec<QuadraticTriangle>,
    Vec<QuadraticEdge>,
) {
    // Initialize parameters
    let mut indices = Vec::<u32>::with_capacity(cells.num_cells() * 3);
    let mut triangle_to_cell_mapping = Vec::new();
    let mut quadratic_triangles = Vec::new();
    let mut quadratic_edges = Vec::new();

    // Unify all format data to (cell_type, vertices) format
    let cell_data = extract_cell_data(cells);

    // Process each cell
    for (cell_idx, (cell_type, vertices)) in cell_data.into_iter().enumerate() {
        process_cell(
            &mut indices,
            &mut triangle_to_cell_mapping,
            &mut quadratic_triangles,
            &mut quadratic_edges,
            cell_idx,
            &cell_type,
            &vertices,
        );
    }

    (
        indices,
        triangle_to_cell_mapping,
        quadratic_triangles,
        quadratic_edges,
    )
}

/// Extract unified format cell data from cells data
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

                // Get vertex count
                let num_vertices = match data_iter.next() {
                    Some(n) => n as usize,
                    None => break,
                };

                // Collect vertex indices
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

                // Extract vertex indices
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

/// Cell processing function
///
/// Perform corresponding triangulation processing based on cell type
fn process_cell(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    quadratic_triangles: &mut Vec<QuadraticTriangle>,
    quadratic_edges: &mut Vec<QuadraticEdge>,
    cell_idx: usize,
    cell_type: &model::CellType,
    vertices: &[u32],
) {
    let initial_index_count = indices.len();

    match cell_type {
        // Basic cell types
        model::CellType::Vertex => {
            validate_vertex_count(vertices, 1, "vertex");
            // Skip vertex element rendering, point elements are not suitable for 3D surface rendering
            println!("Skip vertex element rendering (cell {})", cell_idx);
            // Don't add any rendering indices
        }

        model::CellType::Line => {
            validate_vertex_count(vertices, 2, "line");
            // Skip line element rendering to avoid incorrect visual effects under PBR lighting
            println!("Skip line element rendering (cell {})", cell_idx);
            // Don't add any rendering indices
        }

        model::CellType::Triangle => {
            validate_vertex_count(vertices, 3, "triangle");
            // Directly add triangle indices
            indices.extend(vertices);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Quad => {
            validate_vertex_count(vertices, 4, "quad");
            // Decompose quadrilateral into two triangles
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            triangle_to_cell_mapping.push(cell_idx);
            triangle_to_cell_mapping.push(cell_idx);
        }

        model::CellType::Tetra => {
            validate_vertex_count(vertices, 4, "tetrahedron");
            // Decompose tetrahedron into 4 triangles
            indices.extend_from_slice(&[vertices[0], vertices[1], vertices[2]]);
            indices.extend_from_slice(&[vertices[0], vertices[2], vertices[3]]);
            indices.extend_from_slice(&[vertices[0], vertices[3], vertices[1]]);
            indices.extend_from_slice(&[vertices[1], vertices[3], vertices[2]]);
            for _ in 0..4 {
                triangle_to_cell_mapping.push(cell_idx);
            }
        }

        // Quadratic cell types
        model::CellType::QuadraticEdge => {
            // Skip line element rendering to avoid incorrect visual effects under PBR lighting
            println!("Skip quadratic edge element rendering (cell {})", cell_idx);

            // Save edge data for subsequent subdivision use
            let quadratic_edge = QuadraticEdge::new([
                vertices[0], // p0: r=0 endpoint
                vertices[1], // p1: r=1 endpoint
                vertices[2], // p2: r=0.5 midpoint
            ]);
            quadratic_edges.push(quadratic_edge);

            // Don't add any rendering indices, skip directly
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
            // Try using fan triangulation to process other types
            if vertices.len() >= 3 {
                let fan_indices = triangulate_fan(vertices);
                indices.extend(fan_indices);
                for _ in 0..(vertices.len() - 2) {
                    triangle_to_cell_mapping.push(cell_idx);
                }
            }
        }
    }

    // Verify if mapping is correct
    validate_mapping(
        indices,
        triangle_to_cell_mapping,
        initial_index_count,
        cell_idx,
    );
}

/// Process quadratic edge
#[allow(dead_code)]
fn process_quadratic_edge(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    quadratic_edges: &mut Vec<QuadraticEdge>,
    cell_idx: usize,
    vertices: &[u32],
) {
    validate_vertex_count(vertices, 3, "quadratic edge");

    // Create quadratic edge data structure
    let quadratic_edge = QuadraticEdge::new([
        vertices[0], // p0: r=0 endpoint
        vertices[1], // p1: r=1 endpoint
        vertices[2], // p2: r=0.5 midpoint
    ]);

    // Decompose quadratic edge into two linear edges
    let linear_segments = quadratic_edge.to_linear_segments();

    // Store quadratic edge for subsequent subdivision use
    quadratic_edges.push(quadratic_edge);

    // Convert each linear segment to degenerate triangles for rendering
    for segment in linear_segments {
        indices.extend_from_slice(&[segment[0], segment[1], segment[1]]);
        triangle_to_cell_mapping.push(cell_idx);
    }
}

/// Process quadratic triangle
fn process_quadratic_triangle(
    indices: &mut Vec<u32>,
    triangle_to_cell_mapping: &mut Vec<usize>,
    quadratic_triangles: &mut Vec<QuadraticTriangle>,
    cell_idx: usize,
    vertices: &[u32],
) {
    validate_vertex_count(vertices, 6, "quadratic triangle");

    // Create quadratic triangle data structure (save complete 6 control points information)
    let quadratic_triangle = QuadraticTriangle::new([
        vertices[0],
        vertices[1],
        vertices[2], // Corner vertices
        vertices[3],
        vertices[4],
        vertices[5], // Edge midpoints
    ]);

    // Quadratic triangle rendering only needs corner vertices, edge midpoints are used for subsequent subdivision
    let linear_triangle = quadratic_triangle.to_linear_triangle();
    indices.extend_from_slice(&linear_triangle);
    triangle_to_cell_mapping.push(cell_idx);

    // Store quadratic triangle for subsequent subdivision use
    quadratic_triangles.push(quadratic_triangle);
}

/// Validate vertex count
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

/// Validate if mapping relationship is correct
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
        // Fill missing mappings
        while (triangle_to_cell_mapping.len() - (initial_index_count / 3)) < triangles_added {
            triangle_to_cell_mapping.push(cell_idx);
        }
    }
}
