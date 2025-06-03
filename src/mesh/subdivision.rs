//! # Adaptive Mesh Subdivision Module
//!
//! This module provides infinite subdivision capability for triangular meshes,
//! independent of cell types, using direct geometric subdivision of triangles.
//!
//! ## Core Features
//!
//! - **Infinite Subdivision**: Supports multiple subdivision iterations on any triangular mesh
//! - **Structure Preservation**: Subdivided meshes can be further subdivided indefinitely
//! - **Attribute Interpolation**: Automatically interpolates vertex attribute data
//! - **Memory Optimization**: Edge midpoint caching avoids redundant calculations
//!
//! ## Subdivision Algorithm
//!
//! Uses 4-subdivision algorithm:
//! 1. Create midpoints for each edge of every triangle
//! 2. Decompose original triangle into 4 new smaller triangles
//! 3. Interpolate all vertex attributes
//!
//! ## Usage Example
//!
//! ```rust
//! // Subdivide a mesh once
//! let subdivided_geometry = subdivide_mesh(&geometry)?;
//! ```

use super::vtk::*;
use super::VtkError;
use std::collections::HashMap;

/// Subdivides a mesh using 4-subdivision algorithm
///
/// This is the main subdivision interface that performs one level of subdivision
/// on a triangular mesh. Each subdivision splits every triangle into 4 sub-triangles.
///
/// # Arguments
/// * `geometry` - The geometry data to subdivide
///
/// # Returns
/// * `Ok(GeometryData)` - Subdivided geometry data
/// * `Err(VtkError)` - Error information if subdivision fails
///
/// # Subdivision Rules
/// Each triangle is decomposed into 4 sub-triangles by adding edge midpoints
///
/// # Subdivision Process
/// 1. Validates input mesh is a valid triangular mesh
/// 2. Generates midpoint vertices for each edge
/// 3. Decomposes each original triangle into 4 sub-triangles
/// 4. Interpolates all vertex attributes
/// 5. Updates triangle-to-cell mapping relationships
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

    // Perform mesh subdivision
    let (new_vertices, new_indices, edge_midpoint_map) =
        subdivide_mesh_internal(original_vertices, original_indices)?;

    // Interpolate attribute data
    let new_attributes = interpolate_attributes_for_subdivision(
        geometry,
        &edge_midpoint_map,
        original_vertices.len(),
        new_vertices.len(),
    )?;

    // Generate new triangle-to-cell mapping
    let new_triangle_to_cell_mapping =
        if let Some(original_mapping) = &geometry.triangle_to_cell_mapping {
            generate_subdivided_triangle_mapping(original_mapping)
        } else {
            // If original geometry has no mapping, generate default mapping
            generate_default_triangle_mapping(num_triangles)
        };

    // Create new geometry data
    let new_attributes_converted = new_attributes.into_iter().collect();
    let mut new_geometry = GeometryData::new(new_vertices, new_indices, new_attributes_converted);
    new_geometry.triangle_to_cell_mapping = Some(new_triangle_to_cell_mapping);

    println!(
        "Subdivision completed: {} vertices, {} triangles",
        new_geometry.vertices.len(),
        new_geometry.indices.len() / 3
    );

    Ok(new_geometry)
}

/// Executes 4-subdivision algorithm
///
/// Performs 4-subdivision on each triangle:
/// Original triangle: (v0, v1, v2)
/// New 4 triangles:
/// 1. (v0, mid01, mid20)
/// 2. (mid01, v1, mid12)
/// 3. (mid20, mid12, v2)
/// 4. (mid01, mid12, mid20) - center triangle
///
/// # Arguments
/// * `vertices` - Original vertex list
/// * `indices` - Original index list
///
/// # Returns
/// * `Ok((Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>))` - (new vertex list, new index list, edge midpoint mapping)
/// * `Err(VtkError)` - Error information if subdivision fails
fn subdivide_mesh_internal(
    vertices: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) -> Result<(Vec<[f32; 3]>, Vec<u32>, HashMap<(u32, u32), u32>), VtkError> {
    let num_triangles = indices.len() / 3;

    // Store edges and their corresponding midpoint indices
    let mut edge_midpoints: HashMap<(u32, u32), u32> = HashMap::new();
    let mut new_vertices = vertices.clone();
    let mut new_indices = Vec::with_capacity(num_triangles * 4 * 3); // Each triangle becomes 4

    // Generate 4 sub-triangles for each original triangle
    for triangle_idx in 0..num_triangles {
        let base_idx = triangle_idx * 3;
        let v0 = indices[base_idx];
        let v1 = indices[base_idx + 1];
        let v2 = indices[base_idx + 2];

        // Get or create edge midpoints
        let mid01 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v0, v1);
        let mid12 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v1, v2);
        let mid20 = get_or_create_edge_midpoint(&mut edge_midpoints, &mut new_vertices, v2, v0);

        // Generate 4 sub-triangles
        // Corner triangle 1: (v0, mid01, mid20)
        new_indices.extend_from_slice(&[v0, mid01, mid20]);

        // Corner triangle 2: (mid01, v1, mid12)
        new_indices.extend_from_slice(&[mid01, v1, mid12]);

        // Corner triangle 3: (mid20, mid12, v2)
        new_indices.extend_from_slice(&[mid20, mid12, v2]);

        // Center triangle: (mid01, mid12, mid20)
        new_indices.extend_from_slice(&[mid01, mid12, mid20]);
    }

    Ok((new_vertices, new_indices, edge_midpoints))
}

/// Gets or creates an edge midpoint vertex
///
/// This function implements edge midpoint caching to avoid duplicate calculations.
/// When a midpoint for an edge already exists, it returns the cached index.
/// Otherwise, it creates a new midpoint vertex and caches it.
///
/// # Arguments
/// * `edge_midpoints` - HashMap cache for edge midpoints
/// * `vertices` - Mutable reference to vertex list for adding new midpoints
/// * `v0` - First vertex index of the edge
/// * `v1` - Second vertex index of the edge
///
/// # Returns
/// * `u32` - Index of the midpoint vertex (either existing or newly created)
///
/// # Edge Ordering
/// To ensure consistency, edges are stored with the smaller vertex index first.
/// This prevents duplicate midpoints for the same edge with different vertex ordering.
///
/// # Midpoint Calculation
/// The midpoint is calculated as the arithmetic mean of the two endpoint coordinates:
/// `midpoint = (pos0 + pos1) / 2`
fn get_or_create_edge_midpoint(
    edge_midpoints: &mut HashMap<(u32, u32), u32>,
    vertices: &mut Vec<[f32; 3]>,
    v0: u32,
    v1: u32,
) -> u32 {
    // Ensure consistent edge vertex ordering
    let edge = if v0 < v1 { (v0, v1) } else { (v1, v0) };

    // If midpoint already exists, return it directly
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

/// Interpolates attribute data for subdivision mesh
///
/// This function handles the interpolation of all vertex and cell attributes when
/// subdividing a mesh. It processes both point attributes (which need interpolation
/// for new edge midpoints) and cell attributes (which need expansion since each
/// original cell becomes 4 new triangles).
///
/// # Arguments
/// * `geometry` - Original geometry data containing attributes
/// * `edge_midpoint_map` - Map from edge pairs to their midpoint vertex indices
/// * `_original_vertex_count` - Number of vertices in original mesh (unused but kept for future use)
/// * `new_vertex_count` - Total number of vertices in subdivided mesh
///
/// # Returns
/// * `Ok(HashMap)` - New attribute data with interpolated values
/// * `Err(VtkError)` - Error if interpolation fails
///
/// # Attribute Processing
/// - **Point Attributes**: Interpolated for new edge midpoint vertices
/// - **Cell Attributes**: Expanded so each original cell value is replicated 4 times
///
/// # Supported Attribute Types
/// - Scalar data with lookup tables
/// - Color scalar data (RGB/RGBA)
/// - Vector data (3D vectors)
/// - Tensor data (3x3 matrices)
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
                    // Process point attributes: need to interpolate for new edge midpoints
                    let interpolated_attr = interpolate_point_attribute_for_subdivision(
                        attr,
                        edge_midpoint_map,
                        new_vertex_count,
                    )?;
                    new_attributes.insert((name.clone(), location.clone()), interpolated_attr);
                }
                AttributeLocation::Cell => {
                    // Cell attributes need expansion since each original cell now corresponds to 4 new triangles
                    let expanded_attr = expand_cell_attribute_for_subdivision(attr)?;
                    new_attributes.insert((name.clone(), location.clone()), expanded_attr);
                }
            }
        }
    }

    Ok(new_attributes)
}

/// Interpolates point attribute data for subdivision
///
/// This function handles the interpolation of point (vertex) attributes when new
/// edge midpoint vertices are created during subdivision. Each midpoint vertex
/// gets attribute values interpolated from its two edge endpoint vertices.
///
/// # Arguments
/// * `attr` - Original attribute data to interpolate
/// * `edge_midpoint_map` - Map from edge pairs to their midpoint vertex indices
/// * `new_vertex_count` - Total number of vertices in subdivided mesh
///
/// # Returns
/// * `Ok(AttributeType)` - New attribute data with interpolated values
/// * `Err(VtkError)` - Error if interpolation fails
///
/// # Interpolation Method
/// Uses linear interpolation (arithmetic mean) for all attribute types:
/// `interpolated_value = (value0 + value1) / 2`
///
/// # Supported Attribute Types
/// - **Scalar**: Single value per vertex with optional lookup tables
/// - **ColorScalar**: Multi-component color values (RGB, RGBA, etc.)
/// - **Vector**: 3D vector data (velocity, normals, etc.)
/// - **Tensor**: 3x3 tensor matrices (stress, strain, etc.)
///
/// # Error Handling
/// - Uses default values when original vertex data is missing
/// - Bounds checking for all array accesses
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

            // Interpolate scalar values for each edge midpoint
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

/// Expands cell attribute data for subdivision
///
/// Since each original triangle becomes 4 new triangles during subdivision,
/// cell attributes need to be expanded accordingly. Each original cell's
/// attribute value is replicated 4 times for the 4 new triangles.
///
/// # Arguments
/// * `attr` - Original cell attribute data to expand
///
/// # Returns
/// * `Ok(AttributeType)` - Expanded attribute data with 4x the original size
/// * `Err(VtkError)` - Error if expansion fails
///
/// # Expansion Strategy
/// Each original cell value is simply replicated 4 times:
/// `[value] -> [value, value, value, value]`
///
/// This preserves the original attribute semantics while ensuring each
/// new triangle has the same attribute value as its parent triangle.
///
/// # Supported Attribute Types
/// - **Scalar**: Single values with lookup tables
/// - **ColorScalar**: Multi-component color data
/// - **Vector**: 3D vector data
/// - **Tensor**: 3x3 tensor matrices
fn expand_cell_attribute_for_subdivision(attr: &AttributeType) -> Result<AttributeType, VtkError> {
    match attr {
        AttributeType::Scalar {
            num_comp,
            data,
            table_name,
            lookup_table,
        } => {
            let mut new_data = Vec::with_capacity(data.len() * 4);

            // Replicate each original cell value 4 times
            for &value in data.iter() {
                for _ in 0..4 {
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
            let mut new_data = Vec::with_capacity(data.len() * 4);

            // Replicate each original cell color 4 times
            for color in data.iter() {
                for _ in 0..4 {
                    new_data.push(color.clone());
                }
            }

            Ok(AttributeType::ColorScalar {
                nvalues: *nvalues,
                data: new_data,
            })
        }
        AttributeType::Vector(data) => {
            let mut new_data = Vec::with_capacity(data.len() * 4);

            // Replicate each original cell vector 4 times
            for &vector in data.iter() {
                for _ in 0..4 {
                    new_data.push(vector);
                }
            }

            Ok(AttributeType::Vector(new_data))
        }
        AttributeType::Tensor(data) => {
            let mut new_data = Vec::with_capacity(data.len() * 4);

            // Replicate each original cell tensor 4 times
            for &tensor in data.iter() {
                for _ in 0..4 {
                    new_data.push(tensor);
                }
            }

            Ok(AttributeType::Tensor(new_data))
        }
    }
}

/// Generates triangle-to-cell mapping for subdivided mesh
///
/// After subdivision, each original triangle becomes 4 new triangles.
/// This function creates a mapping from the new triangles back to their
/// original parent cells, preserving the cell association.
///
/// # Arguments
/// * `original_mapping` - Original triangle-to-cell mapping before subdivision
///
/// # Returns
/// * `Vec<usize>` - New mapping with 4x the length, where each original
///   cell index is replicated 4 times for the 4 child triangles
///
/// # Mapping Strategy
/// For each original triangle mapped to cell C, the 4 new triangles
/// are all mapped to the same cell C:
/// ```
/// Original: [triangle0 -> cell0, triangle1 -> cell1, ...]
/// After:    [tri0_0 -> cell0, tri0_1 -> cell0, tri0_2 -> cell0, tri0_3 -> cell0,
///            tri1_0 -> cell1, tri1_1 -> cell1, tri1_2 -> cell1, tri1_3 -> cell1, ...]
/// ```
///
/// This preserves the relationship between mesh triangles and data cells
/// after subdivision operations.
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

/// Generates default triangle-to-cell mapping for subdivided mesh
///
/// When the original geometry data has no triangle-to-cell mapping,
/// this function creates a default mapping where each original triangle
/// is considered its own cell. After subdivision, each set of 4 child
/// triangles maps back to their parent triangle index.
///
/// # Arguments
/// * `num_original_triangles` - Number of triangles in the original mesh
///
/// # Returns
/// * `Vec<usize>` - Default mapping where triangle groups map to sequential indices
///
/// # Default Mapping Strategy
/// ```
/// Original triangles:  [tri0, tri1, tri2, ...]
/// Considered as cells: [cell0, cell1, cell2, ...]
/// After subdivision:   [tri0_0->cell0, tri0_1->cell0, tri0_2->cell0, tri0_3->cell0,
///                       tri1_0->cell1, tri1_1->cell1, tri1_2->cell1, tri1_3->cell1, ...]
/// ```
///
/// This ensures that each group of 4 subdivided triangles maintains
/// association with their original parent triangle.
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
