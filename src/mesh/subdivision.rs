//! # Adaptive Mesh Subdivision Module
//!
//! This module provides subdivision capabilities for triangular meshes, supporting linear triangles (3 vertices), quadratic triangles (6 vertices), and quadratic edges (3 vertices).

use super::{
    AttributeLocation, AttributeType, GeometryData, QuadraticEdge, QuadraticTriangle, VtkError,
};
use bevy::utils::HashMap;

// ============================================================================
// Public Interface
// ============================================================================

/// Subdivide mesh
///
/// This is the main subdivision interface, supporting subdivision of linear and quadratic triangular meshes.
///
/// # Parameters
/// * `geometry` - The geometry data to subdivide
///
/// # Returns
/// * `Ok(GeometryData)` - The subdivided geometry data
/// * `Err(VtkError)` - If subdivision fails, returns error information
///
pub fn subdivide_mesh(geometry: &GeometryData) -> Result<GeometryData, VtkError> {
    let original_vertices = &geometry.vertices;
    let original_indices = &geometry.indices;

    // Validate input
    if original_indices.len() % 3 != 0 {
        return Err(VtkError::InvalidFormat("Mesh must be triangular"));
    }

    let num_triangles = original_indices.len() / 3;

    println!(
        "Starting mesh subdivision, original mesh: {} vertices, {} triangles",
        geometry.vertices.len(),
        num_triangles
    );

    // Execute subdivision
    let (
        new_vertices,
        new_indices,
        edge_midpoint_map,
        new_quadratic_triangles,
        new_quadratic_edges,
    ) = match (&geometry.quadratic_triangles, &geometry.quadratic_edges) {
        (Some(quadratic_triangles), quadratic_edges_opt) => {
            println!(
                "Mesh contains quadratic triangles, using quadratic shape function interpolation"
            );
            // Use quadratic shape function interpolation for subdivision
            let (vertices, indices, edge_map, quad_triangles) =
                quadratic_4_subdivision(original_vertices, original_indices, quadratic_triangles)?;

            // If there are also quadratic edges, subdivide them as well
            let (final_vertices, quad_edges) = if let Some(quadratic_edges) = quadratic_edges_opt {
                println!("Also processing quadratic edge subdivision");
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
            println!("Mesh contains only quadratic edges, using edge shape function interpolation");
            // Process quadratic edge subdivision
            let (edge_vertices, subdivided_edges) =
                quadratic_edge_2_subdivision(original_vertices, quadratic_edges)?;

            // For edge-only cases, also perform regular triangle subdivision
            let (vertices, indices, edge_map) =
                smooth_4_subdivision(&edge_vertices, original_indices)?;

            (vertices, indices, edge_map, Vec::new(), subdivided_edges)
        }
        (None, None) => {
            println!("Mesh contains no quadratic elements, using linear interpolation");
            // Perform standard 4-way subdivision
            let (vertices, indices, edge_map) =
                smooth_4_subdivision(original_vertices, original_indices)?;
            (vertices, indices, edge_map, Vec::new(), Vec::new())
        }
    };

    // Interpolate attribute data
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

    // Generate new triangle to cell mapping
    let new_triangle_to_cell_mapping =
        if let Some(original_mapping) = &geometry.triangle_to_cell_mapping {
            generate_subdivided_triangle_mapping(original_mapping)
        } else {
            // If original geometry data has no mapping, generate default mapping
            generate_default_triangle_mapping(num_triangles)
        };

    // Create new geometry data
    let mut new_geometry = GeometryData::new(new_vertices, new_indices, new_attributes);
    new_geometry.triangle_to_cell_mapping = Some(new_triangle_to_cell_mapping);

    // If there are new quadratic triangles, add them to geometry data
    if !new_quadratic_triangles.is_empty() {
        new_geometry = new_geometry.add_quadratic_triangles(new_quadratic_triangles);
        println!(
            "Generated {} quadratic triangles",
            new_geometry.quadratic_triangles.as_ref().unwrap().len()
        );
    }

    // If there are new quadratic edges, add them to geometry data
    if !new_quadratic_edges.is_empty() {
        new_geometry = new_geometry.add_quadratic_edges(new_quadratic_edges);
        println!(
            "Generated {} quadratic edges",
            new_geometry.quadratic_edges.as_ref().unwrap().len()
        );
    }

    println!(
        "Subdivision completed: {} vertices, {} triangles",
        new_geometry.vertices.len(),
        new_geometry.indices.len() / 3
    );

    Ok(new_geometry)
}

// ============================================================================
// Core Subdivision Algorithms
// ============================================================================

/// Quadratic triangle 4-way subdivision - using quadratic shape function interpolation
///
/// # Parameters
/// * `vertices` - Original vertex list
/// * `indices` - Original index list
/// * `quadratic_triangles` - Quadratic triangle data (containing complete 6 control points)
///
/// # Returns
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>, Vec<QuadraticTriangle>))` - (new vertex list, new index list, edge midpoint mapping, new quadratic triangle list)
/// * `Err(VtkError)` - If subdivision fails, returns error information
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
        "Quadratic triangle subdivision: processing {} quadratic triangles",
        quadratic_triangles.len()
    );

    for (_triangle_idx, quadratic_tri) in quadratic_triangles.iter().enumerate() {
        let corner_verts = quadratic_tri.corner_vertices(); // [v0, v1, v2] - three corner vertices
        let edge_mids = quadratic_tri.edge_midpoints(); // [m01, m12, m20] - three edge midpoints

        // Get corner vertex coordinates
        let p0 = vertices[corner_verts[0] as usize]; // Corner vertex 0
        let p1 = vertices[corner_verts[1] as usize]; // Corner vertex 1
        let p2 = vertices[corner_verts[2] as usize]; // Corner vertex 2

        // Get edge midpoint coordinates
        let p3 = vertices[edge_mids[0] as usize]; // Edge 01 midpoint
        let p4 = vertices[edge_mids[1] as usize]; // Edge 12 midpoint
        let p5 = vertices[edge_mids[2] as usize]; // Edge 20 midpoint

        // Ensure processing quadratic triangles in counter-clockwise order
        // Original quadratic triangle control point order: [v0, v1, v2, m01, m12, m20]

        // Use quadratic shape functions to calculate new edge midpoints
        // Main edge midpoints: midpoints of 3 main edges
        let mid01 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[0],
            corner_verts[1],
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.0), // Edge 01 midpoint parameter coordinates
        );

        let mid12 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[1],
            corner_verts[2],
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.5), // Edge 12 midpoint parameter coordinates
        );

        let mid20 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[2],
            corner_verts[0],
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.5), // Edge 20 midpoint parameter coordinates
        );

        // Calculate midpoints of new edges (edges between sub-triangles)
        // Edge midpoint from mid01 to v0
        let mid_mid01_v0 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[0],
            mid01,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.0), // Subdivided edge midpoint
        );

        // Edge midpoint from mid01 to v1
        let mid_mid01_v1 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid01,
            corner_verts[1],
            &[p0, p1, p2, p3, p4, p5],
            (0.75, 0.0), // Subdivided edge midpoint
        );

        // Edge midpoint from mid12 to v1
        let mid_mid12_v1 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[1],
            mid12,
            &[p0, p1, p2, p3, p4, p5],
            (0.75, 0.25), // Subdivided edge midpoint
        );

        // Edge midpoint from mid12 to v2
        let mid_mid12_v2 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid12,
            corner_verts[2],
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.75), // Subdivided edge midpoint
        );

        // Edge midpoint from mid20 to v2
        let mid_mid20_v2 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            corner_verts[2],
            mid20,
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.75), // Subdivided edge midpoint
        );

        // Edge midpoint from mid20 to v0
        let mid_mid20_v0 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid20,
            corner_verts[0],
            &[p0, p1, p2, p3, p4, p5],
            (0.0, 0.25), // Subdivided edge midpoint
        );

        // Internal connection edge midpoints
        let mid_mid01_mid12 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid01,
            mid12,
            &[p0, p1, p2, p3, p4, p5],
            (0.5, 0.25), // Internal edge midpoint
        );

        let mid_mid12_mid20 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid12,
            mid20,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.5), // Internal edge midpoint
        );

        let mid_mid20_mid01 = get_or_create_quadratic_edge_midpoint(
            &mut edge_midpoints,
            &mut new_vertices,
            mid20,
            mid01,
            &[p0, p1, p2, p3, p4, p5],
            (0.25, 0.25), // Internal edge midpoint
        );

        // Generate indices for 4 sub-triangles (counter-clockwise order)
        // 1. Top-left sub-triangle: (v0, mid01, mid20)
        new_indices.extend_from_slice(&[corner_verts[0], mid01, mid20]);
        let quad_tri_1 = QuadraticTriangle::new([
            corner_verts[0],
            mid01,
            mid20, // Corner vertices
            mid_mid01_v0,
            mid_mid20_mid01,
            mid_mid20_v0, // Edge midpoints
        ]);
        new_quadratic_triangles.push(quad_tri_1);

        // 2. Bottom-right sub-triangle: (mid01, v1, mid12)
        new_indices.extend_from_slice(&[mid01, corner_verts[1], mid12]);
        let quad_tri_2 = QuadraticTriangle::new([
            mid01,
            corner_verts[1],
            mid12, // Corner vertices
            mid_mid01_v1,
            mid_mid12_v1,
            mid_mid01_mid12, // Edge midpoints
        ]);
        new_quadratic_triangles.push(quad_tri_2);

        // 3. Bottom-left sub-triangle: (mid20, mid12, v2)
        new_indices.extend_from_slice(&[mid20, mid12, corner_verts[2]]);
        let quad_tri_3 = QuadraticTriangle::new([
            mid20,
            mid12,
            corner_verts[2], // Corner vertices
            mid_mid12_mid20,
            mid_mid12_v2,
            mid_mid20_v2, // Edge midpoints
        ]);
        new_quadratic_triangles.push(quad_tri_3);

        // 4. Center sub-triangle: (mid01, mid12, mid20) - counter-clockwise order
        new_indices.extend_from_slice(&[mid01, mid12, mid20]);
        let quad_tri_4 = QuadraticTriangle::new([
            mid01,
            mid12,
            mid20, // Corner vertices
            mid_mid01_mid12,
            mid_mid12_mid20,
            mid_mid20_mid01, // Edge midpoints
        ]);
        new_quadratic_triangles.push(quad_tri_4);
    }

    println!(
        "Total generated {} new quadratic triangles",
        new_quadratic_triangles.len()
    );

    Ok((
        new_vertices,
        new_indices,
        edge_midpoints,
        new_quadratic_triangles,
    ))
}

/// Get or create edge midpoint vertex using quadratic shape functions
///
/// This function uses quadratic Lagrange interpolation to calculate edge midpoints
/// It calculates surface positions based on parametric coordinate midpoint positions.
///
/// # Parameters
/// * `edge_midpoints` - Edge midpoint
/// * `vertices` - Vertex list
/// * `v0` - First vertex index of the edge
/// * `v1` - Second vertex index of the edge
/// * `control_points` - 6 control point coordinates of the quadratic triangle
/// * `parametric_coords` - Parametric coordinates (r, s) of the edge midpoint
///
/// # Returns
/// * `u32` - Index of the midpoint vertex
fn get_or_create_quadratic_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
    control_points: &[[f32; 3]; 6],
    parametric_coords: (f32, f32),
) -> u32 {
    // Ensure consistent edge vertex ordering
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // If midpoint already exists, return directly
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    let (r, s) = parametric_coords;

    // Use quadratic shape functions to calculate midpoint coordinates
    let midpoint = quadratic_interpolation(r, s, control_points);

    // Add new vertex
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // Add edge midpoint mapping
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}

/// Quadratic Lagrange interpolation function
///
/// Calculates interpolated point coordinates using quadratic shape functions based on parametric coordinates (r, s) and 6 control points.
///
/// # Parameters
/// * `r` - Parametric coordinate r
/// * `s` - Parametric coordinate s
/// * `control_points` - 6 control point coordinates [p0, p1, p2, p3, p4, p5]
///
/// # Returns
/// * `[f32; 3]` - Interpolated point coordinates
fn quadratic_interpolation(r: f32, s: f32, control_points: &[[f32; 3]; 6]) -> [f32; 3] {
    let t = 1.0 - r - s; // t = 1 - r - s

    // Calculate 6 quadratic shape function values
    let w0 = t * (2.0 * t - 1.0); // W0 = (1-r-s)(2(1-r-s)-1)
    let w1 = r * (2.0 * r - 1.0); // W1 = r(2r-1)
    let w2 = s * (2.0 * s - 1.0); // W2 = s(2s-1)
    let w3 = 4.0 * r * t; // W3 = 4r(1-r-s)
    let w4 = 4.0 * r * s; // W4 = 4rs
    let w5 = 4.0 * s * t; // W5 = 4s(1-r-s)

    // Linear combination to calculate interpolated point coordinates
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] = w0 * control_points[0][i]  // p0 contribution
                  + w1 * control_points[1][i]  // p1 contribution
                  + w2 * control_points[2][i]  // p2 contribution
                  + w3 * control_points[3][i]  // p3 contribution (edge 01 midpoint)
                  + w4 * control_points[4][i]  // p4 contribution (edge 12 midpoint)
                  + w5 * control_points[5][i]; // p5 contribution (edge 20 midpoint)
    }

    result
}

/// Quadratic edge 2-way subdivision - using quadratic edge shape function interpolation
///
/// Implements 2-way subdivision algorithm based on quadratic edge shape functions.
///
/// # Parameters
/// * `vertices` - Original vertex list
/// * `quadratic_edges` - Quadratic edge data (containing complete 3 control points)
///
/// # Returns
/// * `Ok((Vec<[f32; 3]>, Vec<QuadraticEdge>))` - (new vertex list, new quadratic edge list)
/// * `Err(VtkError)` - If subdivision fails, returns error information
fn quadratic_edge_2_subdivision(
    vertices: &Vec<[f32; 3]>,
    quadratic_edges: &Vec<QuadraticEdge>,
) -> Result<(Vec<[f32; 3]>, Vec<QuadraticEdge>), VtkError> {
    let mut new_vertices = vertices.clone();
    let mut new_quadratic_edges = Vec::new();

    println!(
        "Quadratic edge subdivision: processing {} quadratic edges",
        quadratic_edges.len()
    );

    for quadratic_edge in quadratic_edges.iter() {
        // Get control points of the quadratic edge
        let endpoints = quadratic_edge.endpoints();
        let midpoint_idx = quadratic_edge.midpoint();
        let p0 = vertices[endpoints[0] as usize]; // r=0 endpoint
        let p1 = vertices[endpoints[1] as usize]; // r=1 endpoint
        let p2 = vertices[midpoint_idx as usize]; // r=0.5 midpoint

        // Use quadratic edge shape functions to calculate new division points
        // Calculate point at r=0.25 (left half midpoint)
        let left_mid = quadratic_edge_interpolation(0.25, &[p0, p1, p2]);
        let left_mid_idx = new_vertices.len() as u32;
        new_vertices.push(left_mid);

        // Calculate point at r=0.75 (right half midpoint)
        let right_mid = quadratic_edge_interpolation(0.75, &[p0, p1, p2]);
        let right_mid_idx = new_vertices.len() as u32;
        new_vertices.push(right_mid);

        // Generate 2 sub-edges
        // 1. Left half sub-edge: (p0, original midpoint p2, new left_mid)
        let left_edge = QuadraticEdge::new([
            endpoints[0], // p0
            midpoint_idx, // Original midpoint, now endpoint of left half
            left_mid_idx, // New left half midpoint
        ]);
        new_quadratic_edges.push(left_edge);

        // 2. Right half sub-edge: (original midpoint p2, p1, new right_mid)
        let right_edge = QuadraticEdge::new([
            midpoint_idx,  // Original midpoint, now starting point of right half
            endpoints[1],  // p1
            right_mid_idx, // New right half midpoint
        ]);
        new_quadratic_edges.push(right_edge);
    }

    println!(
        "Total generated {} new quadratic edges",
        new_quadratic_edges.len()
    );

    Ok((new_vertices, new_quadratic_edges))
}

/// Quadratic edge Lagrange interpolation function
///
/// Calculates interpolated point coordinates using quadratic edge shape functions based on parametric coordinate r and 3 control points.
///
/// # Parameters
/// * `r` - Parametric coordinate r (0 <= r <= 1)
/// * `control_points` - 3 control point coordinates [p0, p1, p2]
///
/// # Returns
/// * `[f32; 3]` - Interpolated point coordinates
fn quadratic_edge_interpolation(r: f32, control_points: &[[f32; 3]; 3]) -> [f32; 3] {
    // Calculate 3 quadratic edge shape function values
    let w0 = 2.0 * (r - 0.5) * (r - 1.0); // W0 = 2(r-0.5)(r-1)
    let w1 = 2.0 * r * (r - 0.5); // W1 = 2r(r-0.5)
    let w2 = 4.0 * r * (1.0 - r); // W2 = 4r(1-r)

    // Linear combination to calculate interpolated point coordinates
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] = w0 * control_points[0][i]  // p0 contribution
                  + w1 * control_points[1][i]  // p1 contribution
                  + w2 * control_points[2][i]; // p2 contribution (midpoint)
    }

    result
}

/// Standard 4-way subdivision - pure linear interpolation, 1->4 subdivision
///
/// Implements standard 4-way subdivision algorithm, where each triangle is divided into 4 smaller triangles.
///
/// # Parameters
/// * `vertices` - Original vertex list
/// * `indices` - Original index list
///
/// # Returns
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>))` - (new vertex list, new index list, edge midpoint mapping)
/// * `Err(VtkError)` - If subdivision fails, returns error information
fn smooth_4_subdivision(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> Result<(Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>), VtkError> {
    let num_triangles = indices.len() / 3;
    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::with_capacity(num_triangles * 4 * 3);
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();

    for triangle_idx in 0..num_triangles {
        let base_idx = triangle_idx * 3;
        let v0 = indices[base_idx];
        let v1 = indices[base_idx + 1];
        let v2 = indices[base_idx + 2];

        // Get or create linear edge midpoints
        let mid01 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v0, v1);
        let mid12 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v1, v2);
        let mid20 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v2, v0);

        // Standard 4-way subdivision: 4 small triangles
        // 1. (v0, mid01, mid20)
        new_indices.extend_from_slice(&[v0, mid01, mid20]);

        // 2. (mid01, v1, mid12)
        new_indices.extend_from_slice(&[mid01, v1, mid12]);

        // 3. (mid20, mid12, v2)
        new_indices.extend_from_slice(&[mid20, mid12, v2]);

        // 4. (mid01, mid12, mid20) - Center triangle
        new_indices.extend_from_slice(&[mid01, mid12, mid20]);
    }
    Ok((new_vertices, new_indices, edge_midpoints))
}

/// Get or create edge midpoint vertex
///
/// # Parameters
/// * `edge_midpoints` - Edge midpoint
/// * `vertices` - Vertex list for adding new midpoints
/// * `v0` - First vertex index of the edge
/// * `v1` - Second vertex index of the edge
///
/// # Returns
/// * `u32` - Index of the midpoint vertex
///
/// # Edge Ordering
/// Edges are stored with the smaller vertex index first.
///
/// # Midpoint Calculation
/// Midpoint is calculated as the average of two endpoint coordinates:
/// `midpoint = (position0 + position1) / 2`
fn get_or_create_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
) -> u32 {
    // Ensure consistent edge vertex ordering
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // If midpoint already exists, return directly
    if let Some(&midpoint_idx) = edge_midpoints.get(&edge) {
        return midpoint_idx;
    }

    // Calculate midpoint coordinates
    let pos0 = vertices[v0 as usize];
    let pos1 = vertices[v1 as usize];
    let midpoint = [
        (pos0[0] + pos1[0]) * 0.5,
        (pos0[1] + pos1[1]) * 0.5,
        (pos0[2] + pos1[2]) * 0.5,
    ];

    // Add new vertex
    let midpoint_idx = vertices.len() as u32;
    vertices.push(midpoint);

    // Record edge midpoint mapping
    edge_midpoints.insert(edge, midpoint_idx);

    midpoint_idx
}

// ============================================================================
// Attribute Processing
// ============================================================================

/// Interpolate attribute data for subdivided mesh
///
/// This function handles interpolation of all vertex and cell attributes when subdividing meshes.
///
/// # Parameters
/// * `attributes` - HashMap containing attributes
/// * `edge_midpoint_map` - Mapping from edge pairs to their midpoint vertex indices
/// * `_original_vertex_count` - Number of vertices in original mesh
/// * `new_vertex_count` - Total number of vertices in subdivided mesh
///
/// # Returns
/// * `Ok(HashMap)` - New attribute data with interpolated values
/// * `Err(VtkError)` - If interpolation fails, returns error
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
                // Interpolation for new edge midpoint vertices
                let interpolated_attr = interpolate_point_attribute_for_subdivision(
                    attr,
                    edge_midpoint_map,
                    new_vertex_count,
                )?;
                new_attributes.insert((name.clone(), location.clone()), interpolated_attr);
            }
            AttributeLocation::Cell => {
                // Cell attributes need expansion, since each original cell now corresponds to multiple new triangles
                let expansion_factor = 4;
                let expanded_attr = expand_cell_attribute_for_subdivision(attr, expansion_factor)?;
                new_attributes.insert((name.clone(), location.clone()), expanded_attr);
            }
        }
    }

    Ok(new_attributes)
}

/// Interpolate point attribute data for subdivision
///
/// # Parameters
/// * `attr` - Original attribute data to interpolate
/// * `edge_midpoint_map` - Mapping from edge pairs to their midpoint vertex indices
/// * `new_vertex_count` - Total number of vertices in the subdivided mesh
///
/// # Returns
/// * `Ok(AttributeType)` - New attribute data with interpolated values
/// * `Err(VtkError)` - If interpolation fails, returns error
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

            // Check original data range
            let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let range = max_val - min_val;

            if range < 1e-10 {
                // When original data range is extremely small, use constant value
                let avg_val = (min_val + max_val) * 0.5;
                println!(
                    "Original scalar data range is very small ({}), using constant value {} for subdivision",
                    range, avg_val
                );

                // Set the same value for all new edge midpoints
                for (_, &midpoint_idx) in edge_midpoint_map.iter() {
                    if (midpoint_idx as usize) < new_data.len() {
                        new_data[midpoint_idx as usize] = avg_val;
                    }
                }
            } else {
                // Interpolation
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

            // Interpolate color values for each edge midpoint
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

            // Interpolate vector values for each edge midpoint
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

            // Interpolate tensor values for each edge midpoint
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

/// Expand cell attribute data for subdivision
///
/// # Parameters
/// * `attr` - Original cell attribute data to expand
/// * `expansion_factor` - Number of times to replicate original attributes
///
/// # Returns
/// * `Ok(AttributeType)` - Expanded attribute data, size is 4 times the original
/// * `Err(VtkError)` - If expansion fails, returns error
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

            // Each original cell value is replicated 4 times
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

            // Each original cell color is replicated 4 times
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

            // Each original cell vector is replicated 4 times
            for &vector in data.iter() {
                for _ in 0..expansion_factor {
                    new_data.push(vector);
                }
            }

            Ok(AttributeType::Vector(new_data))
        }
        AttributeType::Tensor(data) => {
            let mut new_data = Vec::with_capacity(data.len() * expansion_factor);

            // Each original cell tensor is replicated 4 times
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
// Mapping Processing
// ============================================================================

/// Generate triangle-to-cell mapping for subdivided mesh
///
/// # Parameters
/// * `original_mapping` - Original triangle-to-cell mapping before subdivision
///
/// # Returns
/// * `Vec<usize>` - New mapping with length 4 times the original, where each original cell index is replicated 4 times for 4 sub-triangles

fn generate_subdivided_triangle_mapping(original_mapping: &[usize]) -> Vec<usize> {
    let mut new_mapping = Vec::with_capacity(original_mapping.len() * 4);

    for &cell_idx in original_mapping.iter() {
        // Each original triangle's corresponding cell now corresponds to 4 new triangles
        for _ in 0..4 {
            new_mapping.push(cell_idx);
        }
    }

    new_mapping
}

/// Generate default triangle-to-cell mapping for subdivided mesh
///
/// # Parameters
/// * `num_original_triangles` - Number of triangles in original mesh
///
/// # Returns
/// * `Vec<usize>` - Default mapping where triangle groups map to sequential indices
fn generate_default_triangle_mapping(num_original_triangles: usize) -> Vec<usize> {
    let mut mapping = Vec::with_capacity(num_original_triangles * 4);

    for triangle_idx in 0..num_original_triangles {
        // Each original triangle corresponds to 4 new triangles
        for _ in 0..4 {
            mapping.push(triangle_idx);
        }
    }

    mapping
}
