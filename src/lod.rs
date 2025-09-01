//! # LOD (Level of Detail) System
//!
//! Manages multiple detail levels:
//! - LOD0: Original model (highest precision)
//! - LOD1: Simplified model (50% triangles)
//! - LOD2: Most simplified model (25% triangles)

use crate::mesh::{GeometryData, VtkError};
use crate::ui::UserModelMesh;
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::collections::BTreeMap;

/// LOD level definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LODLevel {
    /// Original precision - highest quality
    LOD0 = 0,
    /// Simplified precision level 1 - medium quality
    LOD1 = 1,
    /// Maximum simplification - lowest quality
    LOD2 = 2,
}

impl LODLevel {
    pub fn distance_threshold(self) -> f32 {
        match self {
            LODLevel::LOD0 => 15.0,
            LODLevel::LOD1 => 30.0,
            LODLevel::LOD2 => f32::MAX,
        }
    }

    pub fn all_levels() -> Vec<LODLevel> {
        vec![LODLevel::LOD0, LODLevel::LOD1, LODLevel::LOD2]
    }
}

/// LOD mesh data container
///
/// Stores the geometry data, mesh handle, and metadata for a specific LOD level.
/// Each LOD level has its own instance of this structure containing the simplified mesh.
#[derive(Debug)]
pub struct LODMeshData {
    /// Geometry data for this LOD level
    pub geometry: GeometryData,
    /// Mesh handle for rendering
    pub mesh_handle: Handle<Mesh>,
    /// Number of triangles in this LOD (for debugging)
    #[allow(dead_code)]
    pub triangle_count: usize,
}

/// LOD manager
///
/// Main component that manages LOD switching for 3D models. Attached to entities
/// that should have automatic level-of-detail management based on camera distance.
///
/// The manager:
/// - Stores multiple LOD versions of the same model
/// - Tracks the current active LOD level
/// - Calculates model bounding box for distance-based scaling
/// - Provides methods for distance-based LOD selection
#[derive(Component)]
pub struct LODManager {
    /// Storage for mesh data at each LOD level
    pub lod_meshes: BTreeMap<LODLevel, LODMeshData>,
    /// Currently active LOD level being rendered
    pub current_lod: LODLevel,
    /// Center point of the model's bounding box
    pub model_center: Vec3,
    /// Size (diagonal length) of the model's bounding box
    pub model_size: f32,
    /// Flag indicating if LOD needs to be updated on next frame
    pub needs_update: bool,
}

impl LODManager {
    /// Create a new LOD manager from original geometry
    ///
    /// Generates simplified versions of the input geometry at different LOD levels
    /// The simplification process uses Quadric Error Metrics (QEM) to maintain
    /// visual quality while reducing triangle count.
    ///
    /// # Parameters
    /// - `original_geometry`: The source geometry to create LOD levels from
    /// - `meshes`: Mutable reference to Bevy's mesh asset storage
    ///
    /// # Returns
    /// - `Ok(LODManager)`: Successfully created LOD manager with all levels
    /// - `Err(VtkError)`: Failed to process geometry or create meshes
    pub fn new(
        original_geometry: GeometryData,
        meshes: &mut ResMut<Assets<Mesh>>,
    ) -> Result<Self, VtkError> {
        let mut lod_meshes = BTreeMap::new();
        let triangle_count = original_geometry.indices.len() / 3;

        println!("Creating LOD manager, original model has {} triangles", triangle_count);

        // Calculate model bounding box
        let (model_center, model_size) = calculate_bounding_box(&original_geometry.vertices);

        // LOD0
        let original_mesh = crate::mesh::create_mesh_from_geometry(&original_geometry);
        let original_handle = meshes.add(original_mesh);
        lod_meshes.insert(
            LODLevel::LOD0,
            LODMeshData {
                geometry: original_geometry.clone(),
                mesh_handle: original_handle,
                triangle_count,
            },
        );
        println!("LOD0 original model complete, {} triangles", triangle_count);

        // LOD1
        if let Ok(simplified_geometry) = simplify_mesh(&original_geometry, 0.5) {
            let simplified_mesh = crate::mesh::create_mesh_from_geometry(&simplified_geometry);
            let simplified_handle = meshes.add(simplified_mesh);
            let simplified_triangle_count = simplified_geometry.indices.len() / 3;

            lod_meshes.insert(
                LODLevel::LOD1,
                LODMeshData {
                    geometry: simplified_geometry,
                    mesh_handle: simplified_handle,
                    triangle_count: simplified_triangle_count,
                },
            );
            println!("LOD1 simplification complete, generated {} triangles", simplified_triangle_count);
        }

        // LOD2
        if let Ok(most_simplified_geometry) = simplify_mesh(&original_geometry, 0.25) {
            let most_simplified_mesh =
                crate::mesh::create_mesh_from_geometry(&most_simplified_geometry);
            let most_simplified_handle = meshes.add(most_simplified_mesh);
            let most_simplified_triangle_count = most_simplified_geometry.indices.len() / 3;

            lod_meshes.insert(
                LODLevel::LOD2,
                LODMeshData {
                    geometry: most_simplified_geometry,
                    mesh_handle: most_simplified_handle,
                    triangle_count: most_simplified_triangle_count,
                },
            );
            println!(
                "LOD2 maximum simplification complete, generated {} triangles",
                most_simplified_triangle_count
            );
        }

        // Initially use LOD0
        let initial_lod = LODLevel::LOD0;

        Ok(LODManager {
            lod_meshes,
            current_lod: initial_lod,
            model_center,
            model_size,
            needs_update: false,
        })
    }

    /// Select appropriate LOD level based on camera distance
    ///
    /// # Parameters
    /// - `distance`: Camera distance to model center
    ///
    /// # Returns
    /// The appropriate LOD level for the given distance
    pub fn select_lod_by_distance(&self, distance: f32) -> LODLevel {
        // Adjust distance thresholds based on model size, use smaller factor for small models
        let size_factor = if self.model_size < 5.0 {    
            (self.model_size / 5.0).max(0.3)
        } else {
            (self.model_size / 10.0).max(1.0)
        };

        for level in LODLevel::all_levels() {
            if self.lod_meshes.contains_key(&level) {
                let threshold = level.distance_threshold() * size_factor;
                if distance <= threshold {
                    return level;
                }
            }
        }

        // Default to lowest precision
        LODLevel::LOD2
    }

    /// Update current LOD level based on camera distance
    ///
    /// # Parameters
    /// - `camera_distance`: Current distance from camera to model center
    ///
    /// # Returns
    /// - `true`: LOD level was changed
    /// - `false`: LOD level remains the same
    pub fn update_lod(&mut self, camera_distance: f32) -> bool {
        let new_lod = self.select_lod_by_distance(camera_distance);
        if new_lod != self.current_lod {
            self.current_lod = new_lod;
            self.needs_update = true;
            
            // Calculate actual distance thresholds used for debugging
            let size_factor = if self.model_size < 5.0 {
                (self.model_size / 5.0).max(0.3)
            } else {
                (self.model_size / 10.0).max(1.0)
            };
            
            println!(
                "LOD switched to {:?}, distance: {:.2}, model size: {:.2}, size factor: {:.2}, LOD0 threshold: {:.2}, LOD1 threshold: {:.2}",
                new_lod, 
                camera_distance, 
                self.model_size,
                size_factor,
                LODLevel::LOD0.distance_threshold() * size_factor,
                LODLevel::LOD1.distance_threshold() * size_factor
            );
            true
        } else {
            false
        }
    }

    /// Get the mesh handle for the current LOD level
    pub fn current_mesh_handle(&self) -> Option<&Handle<Mesh>> {
        self.lod_meshes
            .get(&self.current_lod)
            .map(|data| &data.mesh_handle)
    }

    /// Get the geometry data for the current LOD level
    pub fn current_geometry(&self) -> Option<&GeometryData> {
        self.lod_meshes
            .get(&self.current_lod)
            .map(|data| &data.geometry)
    }
}

/// LOD system plugin
///
/// Adds the LOD management systems to the Bevy app.
pub struct LODPlugin;

impl Plugin for LODPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_lod_based_on_camera_distance,
                update_lod_color_mapping,
            )
                .chain(),
        );
    }
}

/// Update LOD levels based on camera distance
fn update_lod_based_on_camera_distance(
    camera_query: Query<&Transform, (With<Camera3d>, Without<LODManager>)>,
    mut lod_entities: Query<(&mut LODManager, &mut Mesh3d), With<UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    for (mut lod_manager, mut mesh3d) in lod_entities.iter_mut() {
        // Calculate distance from camera to model center
        let distance = camera_transform
            .translation
            .distance(lod_manager.model_center);

        // Update LOD level
        if lod_manager.update_lod(distance) {
            // If LOD level changed, update the mesh
            if let Some(new_mesh_handle) = lod_manager.current_mesh_handle() {
                let mesh_handle_clone = new_mesh_handle.clone();
                *mesh3d = Mesh3d(mesh_handle_clone.clone());
                lod_manager.needs_update = false;

                // Apply current color mapping to the new LOD mesh
                if let (Some(mesh), Some(current_geometry)) = (
                    meshes.get_mut(&mesh_handle_clone),
                    lod_manager.current_geometry(),
                ) {
                    if let Err(e) = crate::ui::color_bar::apply_custom_color_mapping(
                        current_geometry,
                        mesh,
                        &color_bar_config,
                    ) {
                        println!("Failed to apply color mapping to LOD mesh: {:?}", e);
                    }
                }
            }
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate bounding box of vertex array
fn calculate_bounding_box(vertices: &Vec<[f32; 3]>) -> (Vec3, f32) {
    if vertices.is_empty() {
        return (Vec3::ZERO, 1.0);
    }

    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

    for vertex in vertices {
        let v = Vec3::new(vertex[0], vertex[1], vertex[2]);
        min = min.min(v);
        max = max.max(v);
    }

    let center = (min + max) * 0.5;
    let size = (max - min).length();

    (center, size)
}

/// Simplify mesh geometry
fn simplify_mesh(geometry: &GeometryData, ratio: f32) -> Result<GeometryData, VtkError> {
    let ratio = ratio.clamp(0.1, 1.0);
    let original_triangle_count = geometry.indices.len() / 3;
    let target_triangle_count = ((original_triangle_count as f32) * ratio) as usize;

    println!(
        "Simplifying mesh: from {} triangles to {} triangles",
        original_triangle_count, target_triangle_count
    );

    // Use Quadric Error Metrics algorithm for simplification
    simplify_mesh_qem(geometry, ratio)
}

/// Quadric Error Metrics (QEM) based mesh simplification algorithm
///
/// # Algorithm Overview
/// 1. Build half-edge data structure from input geometry
/// 2. Compute quadric matrices for each vertex based on adjacent faces
/// 3. Calculate collapse cost for each edge
/// 4. Iteratively collapse the edge with lowest cost
/// 5. Update affected vertices and recalculate costs
///
/// # Parameters
/// - `geometry`: Source to simplify
/// - `ratio`: Target triangle ratio
///
/// # Returns
/// - `Ok(GeometryData)`: Successfully simplified geometry with preserved attributes
/// - `Err(VtkError)`: Simplification failed or ratio too low
fn simplify_mesh_qem(geometry: &GeometryData, ratio: f32) -> Result<GeometryData, VtkError> {
    if ratio < 0.2 {
        println!("QEM simplification ratio too low ({}), using vertex clustering algorithm", ratio);
        return simplify_mesh_vertex_clustering(geometry, ratio);
    }

    let target_triangle_count = ((geometry.indices.len() / 3) as f32 * ratio) as usize;

    // Build half-edge data structure
    let mut mesh = QEMMesh::from_geometry(geometry);

    // Compute quadric for each vertex
    mesh.compute_vertex_quadrics();

    // Compute collapse cost for all edges
    mesh.compute_edge_costs();

    // Perform edge collapses until target triangle count is reached
    let current_triangle_count = mesh.triangle_count();
    let mut max_collapses = current_triangle_count.saturating_sub(target_triangle_count);

    // Limit maximum collapses to prevent over-simplification
    max_collapses = max_collapses.min(current_triangle_count / 2);

    println!("QEM simplification: planning to collapse at most {} edges", max_collapses);

    let mut collapsed_count = 0;
    let mut consecutive_failures = 0; // Count consecutive failures

    for _i in 0..max_collapses {
        let triangle_count_before = mesh.triangle_count();
        if triangle_count_before <= target_triangle_count {
            break;
        }

        // Prevent over-simplification
        if triangle_count_before < 30 {
            break;
        }

        if !mesh.collapse_cheapest_edge() {
            consecutive_failures += 1;
            if consecutive_failures > 5 {
                break; // Stop after multiple consecutive failures
            }
        } else {
            consecutive_failures = 0; // Reset failure count
            collapsed_count += 1;
        }
    }

    println!(
        "QEM simplification complete: collapsed {} edges, generated {} triangles",
        collapsed_count,
        mesh.triangle_count()
    );

    // Convert back to GeometryData
    mesh.to_geometry_data()
}

/// QEM mesh data structure for simplification
///
/// Internal data structure used during QEM simplification process.
/// Maintains vertices, edges, triangles, and connectivity information
/// needed for the edge collapse algorithm.
struct QEMMesh {
    /// All vertices in the mesh with their quadric matrices
    vertices: Vec<QEMVertex>,
    /// All edges with collapse costs and optimal positions
    edges: Vec<QEMEdge>,
    /// All triangles with plane equations
    triangles: Vec<QEMTriangle>,
    /// Mapping from original vertex indices to QEM vertex indices
    #[allow(dead_code)]
    vertex_mapping: HashMap<usize, usize>,
    /// Preserved cell attributes from original geometry
    original_cell_attributes: Option<HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>>,
}

/// QEM vertex representation
///
/// Represents a vertex in the QEM mesh with its position, quadric matrix,
/// connectivity information, and original attributes for preservation
/// during simplification.
#[derive(Clone)]
struct QEMVertex {
    /// 3D position of the vertex
    position: [f32; 3],
    /// Accumulated quadric matrix for error calculation
    quadric: QuadricMatrix,
    /// Indices of edges connected to this vertex
    edges: Vec<usize>,
    /// Whether this vertex has been deleted during simplification
    is_deleted: bool,
    /// Original vertex attributes to preserve during simplification
    original_attributes: Option<OriginalVertexAttribs>,
}

/// QEM edge representation
///
/// Represents an edge in the QEM mesh with collapse cost information
/// and optimal position calculation for edge collapse operations.
#[derive(Clone)]
struct QEMEdge {
    /// Index of first vertex
    v0: usize,
    /// Index of second vertex  
    v1: usize,
    /// Cost of collapsing this edge
    cost: f32,
    /// Optimal position for collapsed vertex
    optimal_position: [f32; 3],
    /// Triangles that contain this edge
    triangles: Vec<usize>,
    /// Whether this edge has been deleted during simplification
    is_deleted: bool,
}

/// QEM triangle representation
///
/// Represents a triangle in the QEM mesh with its vertices and
/// the plane equation used for quadric matrix calculation.
#[derive(Clone)]
struct QEMTriangle {
    /// Indices of the three vertices forming this triangle
    vertices: [usize; 3],
    /// Plane equation coefficients: ax + by + cz + d = 0
    plane: [f32; 4],
    /// Whether this triangle has been deleted during simplification
    is_deleted: bool,
}

/// Quadric matrix (4x4 symmetric matrix)
///
/// Represents the quadric error matrix used in QEM algorithm.
/// Stores only the upper triangle of the symmetric matrix for efficiency.
/// Used to calculate the geometric error when collapsing edges.
///
/// Matrix layout:
/// ```
/// Q = [q11 q12 q13 q14]
///     [q12 q22 q23 q24]  
///     [q13 q23 q33 q34]
///     [q14 q24 q34 q44]
/// ```
#[derive(Clone)]
struct QuadricMatrix {
    /// Upper triangle elements
    q: [f64; 10],
}

/// Original vertex attributes container
///
/// Stores the original vertex attributes from the source geometry.
#[derive(Clone)]
struct OriginalVertexAttribs {
    /// Scalar attributes
    scalar_values: HashMap<String, f32>,
    /// Vector attributes
    vector_values: HashMap<String, [f32; 3]>,
    /// Color attributes
    color_values: HashMap<String, Vec<f32>>,
}

impl QuadricMatrix {
    /// Create a new zero quadric matrix
    ///
    /// Initializes all matrix elements to zero.
    fn new() -> Self {
        Self { q: [0.0; 10] }
    }

    /// Create quadric matrix from triangle plane equation
    ///
    /// Constructs a quadric matrix from a plane equation (ax + by + cz + d = 0).
    /// The resulting matrix represents the squared distance from the plane.
    ///
    /// # Parameters
    /// - `plane`: Plane coefficients [a, b, c, d] where (a,b,c) is the normal
    ///
    /// # Returns
    /// Quadric matrix representing distance squared to the plane
    fn from_plane(plane: [f32; 4]) -> Self {
        let [a, b, c, d] = plane.map(|x| x as f64);
        Self {
            q: [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ],
        }
    }

    /// Add another quadric matrix to this one
    ///
    /// Accumulates the quadric error by adding the elements of another quadric matrix.
    ///
    /// # Parameters
    /// - `other`: The quadric matrix to add to this one
    fn add(&mut self, other: &QuadricMatrix) {
        for i in 0..10 {
            self.q[i] += other.q[i];
        }
    }

    /// Calculate the quadric error for a vertex position
    ///
    /// Computes the quadric error (squared distance) for placing a vertex
    /// at the given position. Lower values indicate better positions.
    ///
    /// # Parameters
    /// - `v`: Vertex position [x, y, z]
    ///
    /// # Returns
    /// The quadric error value
    fn error(&self, v: [f32; 3]) -> f64 {
        let [x, y, z] = v.map(|x| x as f64);
        let w = 1.0;

        // Calculate v^T * Q * v (quadratic form)
        self.q[0] * x * x
            + 2.0 * self.q[1] * x * y
            + 2.0 * self.q[2] * x * z
            + 2.0 * self.q[3] * x * w
            + self.q[4] * y * y
            + 2.0 * self.q[5] * y * z
            + 2.0 * self.q[6] * y * w
            + self.q[7] * z * z
            + 2.0 * self.q[8] * z * w
            + self.q[9] * w * w
    }

    /// Find optimal position by solving linear system
    ///
    /// Computes the position that minimizes the quadric error by solving
    /// the linear system ∇(v^T Q v) = 0. If the system is singular,
    /// returns None and the caller should try alternative positions.
    ///
    /// # Returns
    /// - `Some([x, y, z])`: Optimal position that minimizes quadric error
    /// - `None`: Matrix is singular, no unique solution exists
    fn optimal_position(&self) -> Option<[f32; 3]> {
        // Build 3x3 matrix A and vector b, solve Ax = -b
        let a11 = self.q[0]; // q11
        let a12 = self.q[1]; // q12
        let a13 = self.q[2]; // q13
        let a22 = self.q[4]; // q22
        let a23 = self.q[5]; // q23
        let a33 = self.q[7]; // q33

        let b1 = self.q[3]; // q14
        let b2 = self.q[6]; // q24
        let b3 = self.q[8]; // q34

        // Calculate determinant
        let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a23 * a13)
            + a13 * (a12 * a23 - a22 * a13);

        // Matrix is singular
        if det.abs() < 1e-12 {
            return None; 
        }

        // Solve using Cramer's rule
        let x = ((-b1) * (a22 * a33 - a23 * a23) - a12 * ((-b2) * a33 - a23 * (-b3))
            + a13 * ((-b2) * a23 - a22 * (-b3)))
            / det;
        let y = (a11 * ((-b2) * a33 - a23 * (-b3)) - (-b1) * (a12 * a33 - a23 * a13)
            + a13 * (a12 * (-b3) - (-b2) * a13))
            / det;
        let z = (a11 * (a22 * (-b3) - (-b2) * a23) - a12 * (a12 * (-b3) - (-b2) * a13)
            + (-b1) * (a12 * a23 - a22 * a13))
            / det;

        Some([x as f32, y as f32, z as f32])
    }
}

impl QEMMesh {
    fn from_geometry(geometry: &GeometryData) -> Self {
        let mut vertices = Vec::new();
        let mut vertex_mapping = HashMap::new();

        // Extract original attributes
        let original_attrs = Self::extract_original_attributes(geometry);

        // Create vertices
        for (i, &pos) in geometry.vertices.iter().enumerate() {
            let attribs = original_attrs.get(&i).cloned();
            vertices.push(QEMVertex {
                position: pos,
                quadric: QuadricMatrix::new(),
                edges: Vec::new(),
                is_deleted: false,
                original_attributes: attribs,
            });
            vertex_mapping.insert(i, i);
        }

        // Create triangles
        let mut triangles = Vec::new();
        for chunk in geometry.indices.chunks(3) {
            if chunk.len() != 3 {
                continue;
            }

            let v0 = chunk[0] as usize;
            let v1 = chunk[1] as usize;
            let v2 = chunk[2] as usize;

            // Calculate triangle normal and plane equation
            let p0 = Vec3::from(vertices[v0].position);
            let p1 = Vec3::from(vertices[v1].position);
            let p2 = Vec3::from(vertices[v2].position);

            let normal = (p1 - p0).cross(p2 - p0).normalize();
            let d = -normal.dot(p0);

            triangles.push(QEMTriangle {
                vertices: [v0, v1, v2],
                plane: [normal.x, normal.y, normal.z, d],
                is_deleted: false,
            });
        }

        // Build edges
        let edges = Self::build_edges(&triangles);

        // Update vertex edge connections
        Self::update_vertex_edges(&mut vertices, &edges);

        // Save original Cell attributes
        let original_cell_attributes = if let Some(ref attrs) = geometry.attributes {
            let mut cell_attrs = HashMap::new();
            for ((name, location), attr_type) in attrs {
                if let crate::mesh::vtk::AttributeLocation::Cell = location {
                    cell_attrs.insert((name.clone(), location.clone()), attr_type.clone());
                }
            }
            if cell_attrs.is_empty() {
                None
            } else {
                Some(cell_attrs)
            }
        } else {
            None
        };

        QEMMesh {
            vertices,
            edges,
            triangles,
            vertex_mapping,
            original_cell_attributes,
        }
    }

    fn extract_original_attributes(
        geometry: &GeometryData,
    ) -> HashMap<usize, OriginalVertexAttribs> {
        let mut attrs_map = HashMap::new();

        if let Some(ref attrs) = geometry.attributes {
            for i in 0..geometry.vertices.len() {
                let mut vertex_attrs = OriginalVertexAttribs {
                    scalar_values: HashMap::new(),
                    vector_values: HashMap::new(),
                    color_values: HashMap::new(),
                };

                for ((name, location), attr_type) in attrs {
                    if let crate::mesh::vtk::AttributeLocation::Point = location {
                        match attr_type {
                            crate::mesh::vtk::AttributeType::Scalar { data, .. } => {
                                if i < data.len() {
                                    vertex_attrs.scalar_values.insert(name.clone(), data[i]);
                                }
                            }
                            crate::mesh::vtk::AttributeType::Vector(vectors) => {
                                if i < vectors.len() {
                                    vertex_attrs.vector_values.insert(name.clone(), vectors[i]);
                                }
                            }
                            crate::mesh::vtk::AttributeType::ColorScalar { data, .. } => {
                                if i < data.len() {
                                    vertex_attrs
                                        .color_values
                                        .insert(name.clone(), data[i].clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                attrs_map.insert(i, vertex_attrs);
            }
        }

        attrs_map
    }

    fn build_edges(triangles: &[QEMTriangle]) -> Vec<QEMEdge> {
        let mut edge_map: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

        // Collect all edges
        for (tri_idx, triangle) in triangles.iter().enumerate() {
            let [v0, v1, v2] = triangle.vertices;
            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                edge_map.entry(edge).or_insert_with(Vec::new).push(tri_idx);
            }
        }

        // Create edge objects
        edge_map
            .into_iter()
            .map(|((v0, v1), triangles)| QEMEdge {
                v0,
                v1,
                cost: f32::INFINITY,
                optimal_position: [0.0; 3],
                triangles,
                is_deleted: false,
            })
            .collect()
    }

    fn update_vertex_edges(vertices: &mut [QEMVertex], edges: &[QEMEdge]) {
        for vertex in vertices.iter_mut() {
            vertex.edges.clear();
        }

        for (edge_idx, edge) in edges.iter().enumerate() {
            if !edge.is_deleted {
                vertices[edge.v0].edges.push(edge_idx);
                vertices[edge.v1].edges.push(edge_idx);
            }
        }
    }

    fn compute_vertex_quadrics(&mut self) {
        // Reset all quadrics
        for vertex in &mut self.vertices {
            vertex.quadric = QuadricMatrix::new();
        }

        // Accumulate each triangle's quadric to its vertices
        for triangle in &self.triangles {
            if triangle.is_deleted {
                continue;
            }

            let quadric = QuadricMatrix::from_plane(triangle.plane);
            for &vertex_idx in &triangle.vertices {
                self.vertices[vertex_idx].quadric.add(&quadric);
            }
        }
    }

    fn compute_edge_costs(&mut self) {
        for edge in &mut self.edges {
            if edge.is_deleted {
                continue;
            }

            let v0_quadric = &self.vertices[edge.v0].quadric;
            let v1_quadric = &self.vertices[edge.v1].quadric;

            // Merge quadrics
            let mut combined_quadric = v0_quadric.clone();
            combined_quadric.add(v1_quadric);

            // Find optimal position
            if let Some(optimal_pos) = combined_quadric.optimal_position() {
                edge.optimal_position = optimal_pos;
                edge.cost = combined_quadric.error(optimal_pos) as f32;
            } else {
                // If unable to solve for optimal position, try endpoint midpoint
                let v0_pos = self.vertices[edge.v0].position;
                let v1_pos = self.vertices[edge.v1].position;
                let midpoint = [
                    (v0_pos[0] + v1_pos[0]) * 0.5,
                    (v0_pos[1] + v1_pos[1]) * 0.5,
                    (v0_pos[2] + v1_pos[2]) * 0.5,
                ];

                let cost_v0 = combined_quadric.error(v0_pos);
                let cost_v1 = combined_quadric.error(v1_pos);
                let cost_mid = combined_quadric.error(midpoint);

                // Choose the position with lowest cost
                if cost_v0 <= cost_v1 && cost_v0 <= cost_mid {
                    edge.optimal_position = v0_pos;
                    edge.cost = cost_v0 as f32;
                } else if cost_v1 <= cost_mid {
                    edge.optimal_position = v1_pos;
                    edge.cost = cost_v1 as f32;
                } else {
                    edge.optimal_position = midpoint;
                    edge.cost = cost_mid as f32;
                }
            }
        }
    }

    fn collapse_cheapest_edge(&mut self) -> bool {
        // Find the edge with lowest cost
        let mut best_edge_idx = None;
        let mut best_cost = f32::INFINITY;

        for (edge_idx, edge) in self.edges.iter().enumerate() {
            if !edge.is_deleted && edge.cost < best_cost {
                best_cost = edge.cost;
                best_edge_idx = Some(edge_idx);
            }
        }

        // No available edges
        let Some(edge_idx) = best_edge_idx else {
            return false; 
        };

        // Execute edge collapse
        self.collapse_edge(edge_idx)
    }

    fn collapse_edge(&mut self, edge_idx: usize) -> bool {
        let edge = self.edges[edge_idx].clone();
        if edge.is_deleted {
            return false;
        }

        let v0_idx = edge.v0;
        let v1_idx = edge.v1;

        // Collapse v1 to v0 and update v0's position
        self.vertices[v0_idx].position = edge.optimal_position;

        // Merge attributes
        if let (Some(ref attrs0), Some(ref attrs1)) = (
            &self.vertices[v0_idx].original_attributes,
            &self.vertices[v1_idx].original_attributes,
        ) {
            let mut merged_attrs = attrs0.clone();

            // Merge scalar attributes
            for (name, &value1) in &attrs1.scalar_values {
                if let Some(&value0) = merged_attrs.scalar_values.get(name) {
                    merged_attrs
                        .scalar_values
                        .insert(name.clone(), (value0 + value1) * 0.5);
                }
            }

            // Merge vector attributes
            for (name, &vector1) in &attrs1.vector_values {
                if let Some(&vector0) = merged_attrs.vector_values.get(name) {
                    let merged = [
                        (vector0[0] + vector1[0]) * 0.5,
                        (vector0[1] + vector1[1]) * 0.5,
                        (vector0[2] + vector1[2]) * 0.5,
                    ];
                    merged_attrs.vector_values.insert(name.clone(), merged);
                }
            }

            self.vertices[v0_idx].original_attributes = Some(merged_attrs);
        }

        // Merge quadrics
        let v1_quadric = self.vertices[v1_idx].quadric.clone();
        self.vertices[v0_idx].quadric.add(&v1_quadric);

        // Update all triangles that reference v1 to reference v0
        for triangle in &mut self.triangles {
            if triangle.is_deleted {
                continue;
            }
            for vertex_ref in &mut triangle.vertices {
                if *vertex_ref == v1_idx {
                    *vertex_ref = v0_idx;
                }
            }
        }

        // Delete triangles containing this edge
        for &tri_idx in &edge.triangles {
            if !self.triangles[tri_idx].is_deleted {
                self.triangles[tri_idx].is_deleted = true;
            }
        }

        // Remove degenerate triangles
        for triangle in &mut self.triangles {
            if triangle.is_deleted {
                continue;
            }
            let [v0, v1, v2] = triangle.vertices;
            if v0 == v1 || v1 == v2 || v2 == v0 {
                triangle.is_deleted = true;
            }
        }

        // Transfer all connections from v1 to v0
        let v1_edges: Vec<usize> = self.vertices[v1_idx].edges.clone();
        for &other_edge_idx in &v1_edges {
            if other_edge_idx == edge_idx {
                continue;
            }

            let other_edge = &mut self.edges[other_edge_idx];
            if other_edge.is_deleted {
                continue;
            }

            if other_edge.v0 == v1_idx {
                other_edge.v0 = v0_idx;
            } else if other_edge.v1 == v1_idx {
                other_edge.v1 = v0_idx;
            }

            // Remove degenerate edges
            if other_edge.v0 == other_edge.v1 {
                other_edge.is_deleted = true;
            }
        }

        // Delete v1 and current edge
        self.vertices[v1_idx].is_deleted = true;
        self.edges[edge_idx].is_deleted = true;

        // Recalculate quadrics and edge costs for affected vertices
        Self::update_vertex_edges(&mut self.vertices, &self.edges);

        // Recalculate edge costs around v0
        let v0_edges: Vec<usize> = self.vertices[v0_idx].edges.clone();
        for &edge_idx in &v0_edges {
            if !self.edges[edge_idx].is_deleted {
                self.compute_single_edge_cost(edge_idx);
            }
        }

        true
    }

    fn compute_single_edge_cost(&mut self, edge_idx: usize) {
        let edge = &mut self.edges[edge_idx];
        if edge.is_deleted {
            return;
        }

        let v0_quadric = &self.vertices[edge.v0].quadric;
        let v1_quadric = &self.vertices[edge.v1].quadric;

        let mut combined_quadric = v0_quadric.clone();
        combined_quadric.add(v1_quadric);

        if let Some(optimal_pos) = combined_quadric.optimal_position() {
            edge.optimal_position = optimal_pos;
            edge.cost = combined_quadric.error(optimal_pos) as f32;
        } else {
            let v0_pos = self.vertices[edge.v0].position;
            let v1_pos = self.vertices[edge.v1].position;
            let midpoint = [
                (v0_pos[0] + v1_pos[0]) * 0.5,
                (v0_pos[1] + v1_pos[1]) * 0.5,
                (v0_pos[2] + v1_pos[2]) * 0.5,
            ];

            let cost_v0 = combined_quadric.error(v0_pos);
            let cost_v1 = combined_quadric.error(v1_pos);
            let cost_mid = combined_quadric.error(midpoint);

            if cost_v0 <= cost_v1 && cost_v0 <= cost_mid {
                edge.optimal_position = v0_pos;
                edge.cost = cost_v0 as f32;
            } else if cost_v1 <= cost_mid {
                edge.optimal_position = v1_pos;
                edge.cost = cost_v1 as f32;
            } else {
                edge.optimal_position = midpoint;
                edge.cost = cost_mid as f32;
            }
        }
    }

    fn triangle_count(&self) -> usize {
        self.triangles.iter().filter(|t| !t.is_deleted).count()
    }

    fn to_geometry_data(&self) -> Result<GeometryData, VtkError> {
        // Collect valid vertices
        let mut vertex_map = HashMap::new();
        let mut new_vertices = Vec::new();

        for (old_idx, vertex) in self.vertices.iter().enumerate() {
            if !vertex.is_deleted {
                let new_idx = new_vertices.len();
                new_vertices.push(vertex.position);
                vertex_map.insert(old_idx, new_idx as u32);
            }
        }

        // Collect valid triangles
        let mut new_indices = Vec::new();
        let mut triangle_to_cell_mapping = Vec::new();
        let mut cell_index = 0;
        
        for triangle in &self.triangles {
            if triangle.is_deleted {
                continue;
            }

            let [v0, v1, v2] = triangle.vertices;
            if let (Some(&new_v0), Some(&new_v1), Some(&new_v2)) = (
                vertex_map.get(&v0),
                vertex_map.get(&v1),
                vertex_map.get(&v2),
            ) {
                new_indices.extend_from_slice(&[new_v0, new_v1, new_v2]);
                triangle_to_cell_mapping.push(cell_index);
                cell_index += 1;
            }
        }

        // Rebuild attributes
        let new_attributes = self.rebuild_attributes(&vertex_map, new_vertices.len())?;

        let mut geometry = GeometryData::new(new_vertices, new_indices, new_attributes);
        
        // Add triangle to cell mapping
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);

        Ok(geometry)
    }

    fn rebuild_attributes(
        &self,
        vertex_map: &HashMap<usize, u32>,
        new_vertex_count: usize,
    ) -> Result<
        HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>,
        VtkError,
    > {
        let mut new_attrs = HashMap::new();

        // Collect all attribute names
        let mut scalar_names = std::collections::HashSet::new();
        let mut vector_names = std::collections::HashSet::new();
        let mut color_names = std::collections::HashSet::new();

        for vertex in &self.vertices {
            if vertex.is_deleted {
                continue;
            }
            if let Some(ref attrs) = vertex.original_attributes {
                scalar_names.extend(attrs.scalar_values.keys().cloned());
                vector_names.extend(attrs.vector_values.keys().cloned());
                color_names.extend(attrs.color_values.keys().cloned());
            }
        }

        // Rebuild scalar attributes
        for name in scalar_names {
            let mut data = vec![0.0; new_vertex_count];
            for (old_idx, vertex) in self.vertices.iter().enumerate() {
                if vertex.is_deleted {
                    continue;
                }
                if let Some(new_idx) = vertex_map.get(&old_idx) {
                    if let Some(ref attrs) = vertex.original_attributes {
                        if let Some(&value) = attrs.scalar_values.get(&name) {
                            data[*new_idx as usize] = value;
                        }
                    }
                }
            }

            let attr = crate::mesh::vtk::AttributeType::Scalar {
                num_comp: 1,
                table_name: "default".to_string(),
                data,
                lookup_table: None,
            };
            new_attrs.insert((name, crate::mesh::vtk::AttributeLocation::Point), attr);
        }

        // Rebuild vector attributes
        for name in vector_names {
            let mut data = vec![[0.0; 3]; new_vertex_count];
            for (old_idx, vertex) in self.vertices.iter().enumerate() {
                if vertex.is_deleted {
                    continue;
                }
                if let Some(new_idx) = vertex_map.get(&old_idx) {
                    if let Some(ref attrs) = vertex.original_attributes {
                        if let Some(&value) = attrs.vector_values.get(&name) {
                            data[*new_idx as usize] = value;
                        }
                    }
                }
            }

            let attr = crate::mesh::vtk::AttributeType::Vector(data);
            new_attrs.insert((name, crate::mesh::vtk::AttributeLocation::Point), attr);
        }

        // Rebuild Cell attributes (handle original Cell attributes)
        if let Some(ref original_cell_attrs) = self.original_cell_attributes {
            for ((name, location), attr_type) in original_cell_attrs {
                let new_triangle_count = self.triangles.iter().filter(|t| !t.is_deleted).count();
                
                match attr_type {
                    crate::mesh::vtk::AttributeType::Scalar { table_name, .. } => {
                        // Assign the same scalar value to each triangle after simplification
                        let mut cell_data = vec![1.0; new_triangle_count]; // default value
                        
                        if let crate::mesh::vtk::AttributeType::Scalar { data: original_data, .. } = attr_type {
                            if !original_data.is_empty() {
                                let default_value = original_data[0]; // Use first value
                                cell_data.fill(default_value);
                                println!("Rebuilding Cell attribute '{}': {} Cells, value={}", name, new_triangle_count, default_value);
                            }
                        }

                        let new_attr = crate::mesh::vtk::AttributeType::Scalar {
                            num_comp: 1,
                            table_name: table_name.clone(),
                            data: cell_data,
                            lookup_table: None,
                        };
                        new_attrs.insert((name.clone(), location.clone()), new_attr);
                    }
                    _ => {
                        // Can be extended to support other Cell attribute types
                    }
                }
            }
        }

        Ok(new_attrs)
    }
}

/// Vertex clustering-based mesh simplification algorithm
fn simplify_mesh_vertex_clustering(
    geometry: &GeometryData,
    ratio: f32,
) -> Result<GeometryData, VtkError> {
    let target_triangle_count = ((geometry.indices.len() / 3) as f32 * ratio) as usize;

    // Calculate bounding box
    let (center, size) = calculate_bounding_box(&geometry.vertices);

    // Create grid resolution
    let grid_resolution = (20.0 * ratio.sqrt()).max(8.0) as usize;
    let cell_size = size / grid_resolution as f32;

    // Grid clustering: merge nearby vertices
    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();

    for (vertex_idx, vertex) in geometry.vertices.iter().enumerate() {
        let pos = Vec3::from(*vertex);
        let grid_pos = (
            ((pos.x - center.x + size * 0.5) / cell_size) as i32,
            ((pos.y - center.y + size * 0.5) / cell_size) as i32,
            ((pos.z - center.z + size * 0.5) / cell_size) as i32,
        );

        grid.entry(grid_pos)
            .or_insert_with(Vec::new)
            .push(vertex_idx);
    }

    // Select representative vertices for each grid cell
    let mut vertex_mapping: HashMap<usize, usize> = HashMap::new();
    let mut new_vertices = Vec::new();
    let mut representative_vertices: HashMap<(i32, i32, i32), usize> = HashMap::new();

    for (grid_pos, vertices_in_cell) in grid.iter() {
        // Select the vertex closest to the grid cell center as the representative
        let cell_center = Vec3::new(
            center.x + (grid_pos.0 as f32 + 0.5) * cell_size - size * 0.5,
            center.y + (grid_pos.1 as f32 + 0.5) * cell_size - size * 0.5,
            center.z + (grid_pos.2 as f32 + 0.5) * cell_size - size * 0.5,
        );

        let representative = vertices_in_cell
            .iter()
            .min_by(|&&a, &&b| {
                let dist_a = Vec3::from(geometry.vertices[a]).distance(cell_center);
                let dist_b = Vec3::from(geometry.vertices[b]).distance(cell_center);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .copied()
            .unwrap();

        let new_vertex_idx = new_vertices.len();
        new_vertices.push(geometry.vertices[representative]);
        representative_vertices.insert(*grid_pos, new_vertex_idx);

        // Map all vertices in this grid cell to the representative vertex
        for &vertex_idx in vertices_in_cell {
            vertex_mapping.insert(vertex_idx, new_vertex_idx);
        }
    }

    // Rebuild triangles, remove duplicate and degenerate triangles
    let mut new_indices = Vec::new();
    let mut triangle_set = std::collections::HashSet::new();

    for chunk in geometry.indices.chunks(3) {
        if chunk.len() != 3 {
            continue;
        }

        let v0 = vertex_mapping[&(chunk[0] as usize)];
        let v1 = vertex_mapping[&(chunk[1] as usize)];
        let v2 = vertex_mapping[&(chunk[2] as usize)];

        // Skip degenerate triangles
        if v0 == v1 || v1 == v2 || v2 == v0 {
            continue;
        }

        // Check triangle quality, avoid too small or degenerate triangles
        let p0 = Vec3::from(new_vertices[v0]);
        let p1 = Vec3::from(new_vertices[v1]);
        let p2 = Vec3::from(new_vertices[v2]);

        // Calculate triangle area
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        let min_area = (size * size) * 1e-6;

        if area < min_area {
            continue;
        }

        // Create normalized triangle
        let mut triangle = [v0, v1, v2];
        triangle.sort();

        if triangle_set.insert(triangle) {
            new_indices.extend_from_slice(&[v0 as u32, v1 as u32, v2 as u32]);
        }

        // Stop adding if target triangle count is reached
        if new_indices.len() / 3 >= target_triangle_count {
            break;
        }
    }

    println!("Simplification complete: actually generated {} triangles", new_indices.len() / 3);

    // Simplify attribute data
    let new_attributes = if let Some(ref attrs) = geometry.attributes {
        simplify_attributes_clustered(attrs, &vertex_mapping, new_vertices.len())?
    } else {
        HashMap::new()
    };

    Ok(GeometryData::new(new_vertices, new_indices, new_attributes))
}

/// Clustering-based attribute simplification
fn simplify_attributes_clustered(
    original_attrs: &HashMap<
        (String, crate::mesh::vtk::AttributeLocation),
        crate::mesh::vtk::AttributeType,
    >,
    vertex_mapping: &HashMap<usize, usize>,
    new_vertex_count: usize,
) -> Result<
    HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>,
    VtkError,
> {
    let mut new_attrs = HashMap::new();

    for ((name, location), attr_type) in original_attrs.iter() {
        match location {
            crate::mesh::vtk::AttributeLocation::Point => {
                let new_attr = simplify_point_attribute_clustered(
                    attr_type,
                    vertex_mapping,
                    new_vertex_count,
                )?;
                new_attrs.insert((name.clone(), location.clone()), new_attr);
            }
            crate::mesh::vtk::AttributeLocation::Cell => {
                println!("Skipping cell attribute '{}' simplification", name);
            }
        }
    }

    Ok(new_attrs)
}

/// Clustering-based point attribute simplification
fn simplify_point_attribute_clustered(
    attr_type: &crate::mesh::vtk::AttributeType,
    vertex_mapping: &HashMap<usize, usize>,
    new_vertex_count: usize,
) -> Result<crate::mesh::vtk::AttributeType, VtkError> {
    use crate::mesh::vtk::AttributeType;

    match attr_type {
        AttributeType::Scalar {
            data,
            num_comp,
            table_name,
            lookup_table,
        } => {
            let mut new_values = vec![0.0; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            // Accumulate attribute values of all original vertices mapped to the same new vertex
            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < data.len() && new_idx < new_values.len() {
                    new_values[new_idx] += data[*old_idx];
                    value_counts[new_idx] += 1;
                }
            }

            // Calculate average values
            for (value, count) in new_values.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    *value /= *count as f32;
                }
            }

            Ok(AttributeType::Scalar {
                num_comp: *num_comp,
                table_name: table_name.clone(),
                data: new_values,
                lookup_table: lookup_table.clone(),
            })
        }
        AttributeType::ColorScalar { nvalues, data } => {
            let mut new_data = vec![vec![0.0; *nvalues as usize]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < data.len() && new_idx < new_data.len() {
                    for (i, &value) in data[*old_idx].iter().enumerate() {
                        if i < new_data[new_idx].len() {
                            new_data[new_idx][i] += value;
                        }
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // Calculate average values
            for (color_data, count) in new_data.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for value in color_data.iter_mut() {
                        *value /= *count as f32;
                    }
                }
            }

            Ok(AttributeType::ColorScalar {
                nvalues: *nvalues,
                data: new_data,
            })
        }
        AttributeType::Vector(vectors) => {
            let mut new_vectors = vec![[0.0; 3]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < vectors.len() && new_idx < new_vectors.len() {
                    for i in 0..3 {
                        new_vectors[new_idx][i] += vectors[*old_idx][i];
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // Calculate average values and normalize vectors
            for (vector, count) in new_vectors.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for component in vector.iter_mut() {
                        *component /= *count as f32;
                    }
                    // Normalize vector length
                    let length =
                        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2])
                            .sqrt();
                    if length > 0.0 {
                        for component in vector.iter_mut() {
                            *component /= length;
                        }
                    }
                }
            }

            Ok(AttributeType::Vector(new_vectors))
        }
        AttributeType::Tensor(tensors) => {
            let mut new_tensors = vec![[0.0; 9]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < tensors.len() && new_idx < new_tensors.len() {
                    for i in 0..9 {
                        new_tensors[new_idx][i] += tensors[*old_idx][i];
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // Calculate average values
            for (tensor, count) in new_tensors.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for component in tensor.iter_mut() {
                        *component /= *count as f32;
                    }
                }
            }

            Ok(AttributeType::Tensor(new_tensors))
        }
    }
}

/// Update LOD mesh colors when color mapping changes
fn update_lod_color_mapping(
    mut lod_entities: Query<&mut LODManager, With<UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Check if color configuration has changed
    if !color_bar_config.has_changed {
        return;
    }

    println!("Color mapping configuration changed, updating all LOD mesh colors");

    for lod_manager in lod_entities.iter_mut() {
        // Update colors for all LOD levels
        for (lod_level, lod_data) in lod_manager.lod_meshes.iter() {
            if let Some(mesh) = meshes.get_mut(&lod_data.mesh_handle) {
                if let Err(e) = crate::ui::color_bar::apply_custom_color_mapping(
                    &lod_data.geometry,
                    mesh,
                    &color_bar_config,
                ) {
                    println!("Unable to apply color mapping for {:?} level: {:?}", lod_level, e);
                }
            }
        }
    }
}
