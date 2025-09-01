use std::fmt;
pub mod color_maps;
pub mod subdivision;
pub mod triangulation;
pub mod vtk;
pub mod wave;
pub use self::vtk::{AttributeLocation, AttributeType};
// pub use self::color_maps::{ColorMapper, ColorMappingConfig};

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::utils::HashMap;
// use vtkio::*;

// ============================================================================
// Core geometry data structures
// ============================================================================
/// Quadratic edge data structure
///
/// Quadratic edges contain 3 control points: 2 endpoints and 1 edge midpoint
#[derive(Debug, Clone)]
pub struct QuadraticEdge {
    /// Index of 3 control points: [p0, p1, p2]
    /// p0: r=0 endpoint, p1: r=1 endpoint, p2: r=0.5 midpoint
    pub vertices: [u32; 3],
}

/// Quadratic triangle data structure
///
/// Quadratic triangles contain 6 control points: 3 corner points and 3 edge midpoints
/// Vertex layout:
/// - vertices[0,1,2]: Three corner vertices
/// - vertices[3]: Midpoint of edge 0-1
/// - vertices[4]: Midpoint of edge 1-2  
/// - vertices[5]: Midpoint of edge 2-0
#[derive(Debug, Clone)]
pub struct QuadraticTriangle {
    /// Indices of 6 control points: [v0, v1, v2, m01, m12, m20]
    pub vertices: [u32; 6],
}

impl QuadraticEdge {
    /// Create a new quadratic edge
    pub fn new(vertices: [u32; 3]) -> Self {
        Self { vertices }
    }

    /// Get endpoint indices (for rendering)
    pub fn endpoints(&self) -> [u32; 2] {
        [self.vertices[0], self.vertices[1]]
    }

    /// Get midpoint index (for subdivision)
    pub fn midpoint(&self) -> u32 {
        self.vertices[2]
    }

    /// Get all vertex indices
    // pub fn all_vertices(&self) -> [u32; 3] {
    //     self.vertices
    // }

    /// Convert to linear edge segments (split into two segments)
    #[allow(dead_code)]
    pub fn to_linear_segments(&self) -> [[u32; 2]; 2] {
        [
            [self.vertices[0], self.vertices[2]], // First segment: p0 to p2
            [self.vertices[2], self.vertices[1]], // Second segment: p2 to p1
        ]
    }
}

impl QuadraticTriangle {
    /// Create a new quadratic triangle
    pub fn new(vertices: [u32; 6]) -> Self {
        Self { vertices }
    }

    /// Get corner vertex indices (for rendering)
    pub fn corner_vertices(&self) -> [u32; 3] {
        [self.vertices[0], self.vertices[1], self.vertices[2]]
    }

    /// Get edge midpoint indices (for subdivision)
    pub fn edge_midpoints(&self) -> [u32; 3] {
        [self.vertices[3], self.vertices[4], self.vertices[5]]
    }

    /// Get all vertex indices
    #[allow(dead_code)]
    pub fn all_vertices(&self) -> [u32; 6] {
        self.vertices
    }

    /// Convert to linear triangle (using corner vertices only)
    pub fn to_linear_triangle(&self) -> [u32; 3] {
        self.corner_vertices()
    }
}

/// Core geometry data structure
///
/// Contains all geometric information and attribute data for meshes, supporting linear and quadratic meshes
#[derive(Clone, Debug)]
pub struct GeometryData {
    /// Vertex coordinates
    pub vertices: Vec<[f32; 3]>,
    /// Triangle indices
    pub indices: Vec<u32>,
    /// Attribute data
    pub attributes: Option<HashMap<(String, AttributeLocation), AttributeType>>,
    /// Lookup table data
    pub lookup_tables: HashMap<String, Vec<[f32; 4]>>,
    /// Normal vectors
    #[allow(dead_code)]
    normals: Option<Vec<[f32; 3]>>,
    /// Mapping from triangles to original cells
    pub triangle_to_cell_mapping: Option<Vec<usize>>,
    /// Quadratic triangle data for subdivision
    pub quadratic_triangles: Option<Vec<QuadraticTriangle>>,
    /// Quadratic edge data for subdivision
    pub quadratic_edges: Option<Vec<QuadraticEdge>>,
}

#[allow(dead_code)]
impl GeometryData {
    /// Create new geometry data
    pub fn new(
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
        attributes: HashMap<(String, AttributeLocation), AttributeType>,
    ) -> Self {
        Self {
            vertices,
            indices,
            attributes: Some(attributes),
            lookup_tables: HashMap::new(),
            normals: None,
            triangle_to_cell_mapping: None,
            quadratic_triangles: None,
            quadratic_edges: None,
        }
    }

    /// Add quadratic triangle data
    pub fn add_quadratic_triangles(mut self, quadratic_triangles: Vec<QuadraticTriangle>) -> Self {
        self.quadratic_triangles = Some(quadratic_triangles);
        self
    }

    /// Add quadratic edge data
    pub fn add_quadratic_edges(mut self, quadratic_edges: Vec<QuadraticEdge>) -> Self {
        self.quadratic_edges = Some(quadratic_edges);
        self
    }

    /// Add attribute data
    pub fn add_attributes(
        mut self,
        attributes: HashMap<(String, AttributeLocation), AttributeType>,
    ) -> Self {
        self.attributes = Some(attributes);
        self
    }

    /// Add triangle to cell mapping
    pub fn add_triangle_to_cell_mapping(mut self, mapping: Vec<usize>) -> Self {
        self.triangle_to_cell_mapping = Some(mapping);
        self
    }

    /// Get attribute data
    pub fn get_attributes(
        &self,
        name: &str,
        location: AttributeLocation,
    ) -> Option<&AttributeType> {
        self.attributes.as_ref()?.get(&(name.to_string(), location))
    }

    /// Add lookup table
    pub fn add_lookup_table(&mut self, name: String, colors: Vec<[f32; 4]>) {
        self.lookup_tables.insert(name, colors);
    }
}

// ============================================================================
// Error type definitions
// ============================================================================

/// Error types that can occur during VTK file processing
///
/// This enumeration defines various error conditions that may be encountered
/// when parsing and processing VTK format files.
#[derive(Debug)]
#[allow(dead_code)]
pub enum VtkError {
    /// VTK file loading failed
    ///
    /// This error is returned when a VTK file cannot be read or parsed.
    LoadError(String),

    /// Invalid VTK file format
    ///
    /// This error is returned when the VTK file format does not meet expected standards.
    InvalidFormat(&'static str),

    /// Unsupported data type
    ///
    /// This error is returned when encountering VTK data types not supported by current implementation.
    UnsupportedDataType,

    /// Missing required data
    ///
    /// This error is returned when VTK file lacks key data required for processing.
    MissingData(&'static str),

    /// Array index out of bounds
    ///
    /// This error is returned when accessing arrays or lists with indices beyond valid range.
    IndexOutOfBounds {
        /// The index that was attempted to be accessed
        index: usize,
        /// The maximum valid index of the array
        max: usize,
    },

    /// Data type mismatch
    ///
    /// This error is returned when expected data type doesn't match actual encountered data type.
    DataTypeMismatch {
        /// The expected data type
        expected: &'static str,
        /// The actual encountered data type
        found: &'static str,
    },

    /// Attribute size mismatch
    ///
    /// This error is returned when attribute data size doesn't match expected size.
    AttributeMismatch {
        /// The actual attribute size
        attribute_size: usize,
        /// The expected attribute size
        expected_size: usize,
    },

    /// Data conversion error
    ConversionError(String),

    /// I/O operation error
    IoError(std::io::Error),

    /// Generic error
    ///
    /// Used to handle other uncategorized error conditions.
    GenericError(String),
}

// Implements Display trait for VtkError
impl fmt::Display for VtkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VtkError::LoadError(msg) => write!(f, "Load VTK file error: {}", msg),
            VtkError::InvalidFormat(detail) => write!(f, "Invalid VTK format: {}", detail),
            VtkError::UnsupportedDataType => write!(f, "Unsupported data type"),
            VtkError::MissingData(what) => write!(f, "Missing data: {}", what),
            VtkError::IndexOutOfBounds { index, max } => {
                write!(f, "Index out of bounds: {} (max is {})", index, max)
            }
            VtkError::DataTypeMismatch { expected, found } => {
                write!(
                    f,
                    "Data type mismatch: expected {}, found {}",
                    expected, found
                )
            }
            VtkError::AttributeMismatch {
                attribute_size,
                expected_size,
            } => {
                write!(
                    f,
                    "Attribute size mismatch: attribute size {}, expected {}",
                    attribute_size, expected_size
                )
            }
            VtkError::ConversionError(msg) => write!(f, "Conversion error: {}", msg),
            VtkError::IoError(err) => write!(f, "IO error: {}", err),
            VtkError::GenericError(msg) => write!(f, "Error: {}", msg),
        }
    }
}

// Implements standard error trait for VtkError
impl std::error::Error for VtkError {}

/// Implements automatic conversion from io::Error to VtkError
///
/// This allows automatic conversion of standard library io::Error to VtkError::IoError
/// when handling file I/O operations, simplifying error handling code.
impl From<std::io::Error> for VtkError {
    fn from(err: std::io::Error) -> Self {
        VtkError::IoError(err)
    }
}

// Prints basic VTK file information to console
// pub fn print_vtk_info(vtk: &Vtk) {
//     println!("VTK file information:");
//     println!("  Version: {:?}", vtk.version);
//     println!("  Title: {}", vtk.title);

//     match &vtk.data {
//         model::DataSet::UnstructuredGrid { meta, pieces } => {
//             println!("  Data type: UnstructuredGrid");
//             println!("  Meta data: {:?}", meta);
//             println!("  Pieces number: {}", pieces.len());
//         }
//         model::DataSet::PolyData { meta, pieces } => {
//             println!("  Data type: PolyData");
//             println!("  Meta data: {:?}", meta);
//             println!("  Pieces number: {}", pieces.len());
//         }
//         _ => println!("  Data type: Other"),
//     }
// }

// Print detailed geometry data information to console
// pub fn print_geometry_info(geometry: &GeometryData) {
//     println!("Geometry data information:");
//     println!("  Vertex number: {}", geometry.vertices.len());
//     println!("  Index number: {}", geometry.indices.len());
//     println!("  Triangle number: {}", geometry.indices.len() / 3);

//     if let Some(attributes) = &geometry.attributes {
//         println!("  Attribute number: {}", attributes.len());

//         for ((name, location), attr) in attributes.iter() {
//             match attr {
//                 AttributeType::Scalar {
//                     num_comp,
//                     table_name,
//                     data,
//                     lookup_table,
//                 } => {
//                     println!("  Scalar attribute: {} (location: {:?})", name, location);
//                     println!("    Component number: {}", num_comp);
//                     println!("    Lookup table name: {}", table_name);
//                     println!("    Data length: {}", data.len());
//                     if let Some(lut) = lookup_table {
//                         println!("    Lookup table color number: {}", lut.len());
//                     }
//                 }
//                 AttributeType::ColorScalar { nvalues, data } => {
//                     println!(
//                         "  Color scalar attribute: {} (location: {:?})",
//                         name, location
//                     );
//                     println!("    Value number: {}", nvalues);
//                     println!("    Data length: {}", data.len());
//                 }
//                 AttributeType::Vector(data) => {
//                     println!("  Vector attribute: {} (location: {:?})", name, location);
//                     println!("    Data length: {}", data.len());
//                 }
//                 // other types (such as Tensor) will be added in future development
//                 _ => {
//                     println!(
//                         "  Other attribute: {} (location: {:?}) - logic to be implemented",
//                         name, location
//                     );
//                 }
//             }
//         }
//     } else {
//         println!("  No attributes");
//     }

//     println!("  Lookup table number: {}", geometry.lookup_tables.len());
//     for (name, colors) in &geometry.lookup_tables {
//         println!("  Lookup table: {} (color number: {})", name, colors.len());
//         if !colors.is_empty() {
//             println!(
//                 "    First color: [{:.2}, {:.2}, {:.2}, {:.2}]",
//                 colors[0][0], colors[0][1], colors[0][2], colors[0][3]
//             );
//             println!(
//                 "    Last color: [{:.2}, {:.2}, {:.2}, {:.2}]",
//                 colors[colors.len() - 1][0],
//                 colors[colors.len() - 1][1],
//                 colors[colors.len() - 1][2],
//                 colors[colors.len() - 1][3]
//             );
//         }
//     }
// }

//************************************* Main Process Logic**************************************//

// Creates an optimized Bevy rendering mesh from geometry data
pub fn create_mesh_from_geometry(geometry: &GeometryData) -> Mesh {
    // 1. create a basic mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // 2. add vertex positions
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(geometry.vertices.clone()),
    );

    // 3. add vertex indices
    mesh.insert_indices(Indices::U32(geometry.indices.clone()));

    // 4. compute normals
    mesh.compute_normals();

    // 5. apply color attributes by priority
    // 5.1 first try to apply scalar attributes (typically the most important data)
    let scalar_applied = geometry.apply_scalar_attributes(&mut mesh).is_ok();
    println!("Scalar attributes applied: {}", scalar_applied);

    // 5.2 if no scalar attributes, try to apply cell color
    if !scalar_applied {
        let cell_color_applied = geometry.apply_cell_color_scalars(&mut mesh).is_ok();
        println!("Cell color attributes applied: {}", cell_color_applied);

        // 5.3 if no cell color, try to apply point color
        if !cell_color_applied {
            let point_color_applied = geometry.apply_point_color_scalars(&mut mesh).is_ok();
            println!("Point color attributes applied: {}", point_color_applied);

            // 5.4 if no color attributes, apply default colors
            if !point_color_applied {
                println!("No color attributes found, applying default colors");
                // default use white
                let default_colors = vec![[1.0, 1.0, 1.0, 1.0]; geometry.vertices.len()];
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    VertexAttributeValues::from(default_colors),
                );
            }
        }
    }

    mesh
}
