use std::fmt;
pub mod color_maps;
pub mod subdivision;
pub mod triangulation;
pub mod vtk;
use self::vtk::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use vtkio::*;

/// Error types that can occur during VTK file processing
///
/// This enumeration defines various error conditions that may be encountered
/// when parsing and processing VTK format files, including file reading errors,
/// format incompatibility errors, data type errors, etc.
///
/// # Examples
///
/// ```rust
/// use mesh::VtkError;
///
/// let error = VtkError::LoadError("File not found".to_string());
/// println!("Error: {}", error);
/// ```
#[derive(Debug)]
#[allow(dead_code)]
pub enum VtkError {
    /// VTK file loading failed
    ///
    /// This error is returned when a VTK file cannot be read or parsed.
    /// Usually caused by file not existing, insufficient permissions, or corrupted file.
    LoadError(String),

    /// Invalid VTK file format
    ///
    /// This error is returned when the VTK file format does not meet expected standards.
    /// For example, missing necessary header information, unsupported format version, etc.
    InvalidFormat(&'static str),

    /// Unsupported data type
    ///
    /// This error is returned when encountering VTK data types not supported by current implementation.
    /// For example, certain advanced cell types or special data structures.
    UnsupportedDataType,

    /// Missing required data
    ///
    /// This error is returned when VTK file lacks key data required for processing.
    /// For example, missing vertex coordinates, cell definitions, etc.
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
    /// For example, when vertex attribute count doesn't match vertex count.
    AttributeMismatch {
        /// The actual attribute size
        attribute_size: usize,
        /// The expected attribute size
        expected_size: usize,
    },

    /// Data conversion error
    ///
    /// This error is returned when errors occur during data format conversion.
    /// For example, type conversion failures, numeric range overflow, etc.
    ConversionError(String),

    /// I/O operation error
    ///
    /// This error is returned when underlying file system operations fail.
    /// This is a wrapper around the standard library's io::Error.
    IoError(std::io::Error),

    /// Generic error
    ///
    /// Used to handle other uncategorized error conditions.
    GenericError(String),
}

/// Implements Display trait for VtkError to provide user-friendly error messages
///
/// This implementation ensures error messages can be displayed in a readable format,
/// facilitating debugging and error reporting in user interfaces.
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

/// Implements standard error trait for VtkError
///
/// This makes VtkError compatible with Rust's error handling ecosystem,
/// supporting error chain propagation and standard error handling patterns.
impl std::error::Error for VtkError {}

/// Implements automatic conversion from io::Error to VtkError
///
/// This allows automatic conversion of standard library io::Error to VtkError::IoError
/// when handling file I/O operations, simplifying error handling code.
///
/// # Examples
///
/// ```rust
/// use std::fs::File;
/// use mesh::VtkError;
///
/// fn read_file() -> Result<(), VtkError> {
///     let _file = File::open("data.vtk")?; // io::Error automatically converts to VtkError
///     Ok(())
/// }
/// ```
impl From<std::io::Error> for VtkError {
    fn from(err: std::io::Error) -> Self {
        VtkError::IoError(err)
    }
}

/// Prints basic VTK file information to console
///
/// This utility function outputs VTK file metadata for debugging and understanding file structure.
/// The output includes file version, title, data type, and data piece count information.
///
/// # Arguments
///
/// * `vtk` - Reference to a parsed VTK file object
///
/// # Output Information
///
/// Prints the following information to console:
/// - **Version**: VTK file format version (Legacy or XML)
/// - **Title**: Title information from file header
/// - **Data Type**: Such as UnstructuredGrid, PolyData, etc.
/// - **Metadata**: Metadata related to the dataset
/// - **Piece Count**: Number of data pieces the data is divided into
///
/// # Supported Data Types
///
/// - `UnstructuredGrid` - Unstructured grid data
/// - `PolyData` - Polygon data
/// - Other types will be displayed as "Other"
///
/// # Examples
///
/// ```rust
/// use vtkio::Vtk;
/// use mesh::print_vtk_info;
///
/// let vtk = Vtk::import("model.vtk").unwrap();
/// print_vtk_info(&vtk);
/// // Output:
/// // VTK file information:
/// //   Version: Legacy { major: 2, minor: 0 }
/// //   Title: Sample mesh data
/// //   Data type: UnstructuredGrid
/// //   Meta data: None
/// //   Pieces number: 1
/// ```
///
/// # Use Cases
///
/// - **Debugging**: Verify if VTK file is loaded correctly
/// - **Inspection**: Understand data structure and content overview
/// - **Logging**: Record file information during processing
pub fn print_vtk_info(vtk: &Vtk) {
    println!("VTK file information:");
    println!("  Version: {:?}", vtk.version);
    println!("  Title: {}", vtk.title);

    match &vtk.data {
        model::DataSet::UnstructuredGrid { meta, pieces } => {
            println!("  Data type: UnstructuredGrid");
            println!("  Meta data: {:?}", meta);
            println!("  Pieces number: {}", pieces.len());
        }
        model::DataSet::PolyData { meta, pieces } => {
            println!("  Data type: PolyData");
            println!("  Meta data: {:?}", meta);
            println!("  Pieces number: {}", pieces.len());
        }
        _ => println!("  Data type: Other"),
    }
}

/// Prints detailed geometry data information to console
///
/// This utility function provides comprehensive analysis output of geometry data, including
/// mesh statistics, attribute details, and color table information. Very useful for debugging
/// mesh processing workflows and verifying data integrity.
///
/// # Arguments
///
/// * `geometry` - Reference to the geometry data object to analyze
///
/// # Output Information
///
/// ## Basic Statistics
/// - **Vertex Count**: Total number of vertices in the mesh
/// - **Index Count**: Total number of indices used to define faces
/// - **Triangle Count**: Calculated number of triangles (index count / 3)
///
/// ## Attribute Information
/// For each attribute, displays:
/// - **Scalar Attributes**: Name, location, component count, lookup table name, data length
/// - **Color Scalars**: Name, location, value count, data length  
/// - **Vector Attributes**: Name, location, data length
///
/// ## Lookup Table Information
/// - Lookup table count and names
/// - Color count for each table
/// - First and last color value examples (RGBA format)
///
/// # Attribute Location Types
///
/// - `Point` - Vertex attributes, one value per vertex
/// - `Cell` - Cell attributes, one value per face/cell
///
/// # Examples
///
/// ```rust
/// use mesh::{print_geometry_info, vtk::GeometryData};
///
/// let geometry: GeometryData = // ... obtained from somewhere
/// print_geometry_info(&geometry);
///
/// // Example output:
/// // Geometry data information:
/// //   Vertex number: 160
/// //   Index number: 984  
/// //   Triangle number: 328
/// //   Attribute number: 2
/// //   Scalar attribute: temperature (location: Point)
/// //     Component number: 1
/// //     Lookup table name: default
/// //     Data length: 160
/// //   Color scalar attribute: material (location: Cell)  
/// //     Value number: 3
/// //     Data length: 328
/// //   Lookup table number: 1
/// //   Lookup table: rainbow (color number: 256)
/// //     First color: [0.00, 0.00, 1.00, 1.00]
/// //     Last color: [1.00, 0.00, 0.00, 1.00]
/// ```
///
/// # Use Cases
///
/// - **Debugging**: Verify mesh parsing correctness
/// - **Analysis**: Understand data scale and complexity
/// - **Validation**: Confirm attribute and color table integrity
/// - **Optimization**: Assess memory usage and processing complexity
pub fn print_geometry_info(geometry: &GeometryData) {
    println!("Geometry data information:");
    println!("  Vertex number: {}", geometry.vertices.len());
    println!("  Index number: {}", geometry.indices.len());
    println!("  Triangle number: {}", geometry.indices.len() / 3);

    if let Some(attributes) = &geometry.attributes {
        println!("  Attribute number: {}", attributes.len());

        for ((name, location), attr) in attributes.iter() {
            match attr {
                AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                    lookup_table,
                } => {
                    println!("  Scalar attribute: {} (location: {:?})", name, location);
                    println!("    Component number: {}", num_comp);
                    println!("    Lookup table name: {}", table_name);
                    println!("    Data length: {}", data.len());
                    if let Some(lut) = lookup_table {
                        println!("    Lookup table color number: {}", lut.len());
                    }
                }
                AttributeType::ColorScalar { nvalues, data } => {
                    println!(
                        "  Color scalar attribute: {} (location: {:?})",
                        name, location
                    );
                    println!("    Value number: {}", nvalues);
                    println!("    Data length: {}", data.len());
                }
                AttributeType::Vector(data) => {
                    println!("  Vector attribute: {} (location: {:?})", name, location);
                    println!("    Data length: {}", data.len());
                }
                // other types (such as Tensor) will be added in future development
                _ => {
                    println!(
                        "  Other attribute: {} (location: {:?}) - logic to be implemented",
                        name, location
                    );
                }
            }
        }
    } else {
        println!("  No attributes");
    }

    println!("  Lookup table number: {}", geometry.lookup_tables.len());
    for (name, colors) in &geometry.lookup_tables {
        println!("  Lookup table: {} (color number: {})", name, colors.len());
        if !colors.is_empty() {
            println!(
                "    First color: [{:.2}, {:.2}, {:.2}, {:.2}]",
                colors[0][0], colors[0][1], colors[0][2], colors[0][3]
            );
            println!(
                "    Last color: [{:.2}, {:.2}, {:.2}, {:.2}]",
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                colors[colors.len() - 1][3]
            );
        }
    }
}

//************************************* Main Process Logic**************************************//

/// Creates an optimized Bevy rendering mesh from geometry data
///
/// This function creates a Mesh object suitable for Bevy engine rendering using
/// pre-parsed GeometryData. This approach is more efficient than re-parsing VTK files,
/// avoiding redundant I/O and parsing overhead.
///
/// The function automatically applies color attributes in priority order:
/// 1. Scalar attributes (typically the most important data)
/// 2. Cell colors
/// 3. Point colors  
/// 4. Default white color (if no color attributes are available)
///
/// # Arguments
///
/// * `geometry` - Pre-parsed geometry data containing vertices, indices, and attribute information
///
/// # Returns
///
/// Returns a fully configured Bevy Mesh object containing:
/// - Vertex positions (ATTRIBUTE_POSITION)
/// - Triangle indices
/// - Computed normals
/// - Color attributes (if available)
///
/// # Features
///
/// - **Complete**: Automatic normal calculation and color attribute application
/// - **Compatible**: Generated Mesh is directly compatible with Bevy rendering system
/// - **Fault-tolerant**: Automatically applies default colors when color attributes are missing
///
/// # Examples
///
/// ```rust
/// use mesh::{create_mesh_from_geometry, vtk::GeometryData};
/// use bevy::prelude::*;
///
/// // Assume we have parsed geometry data
/// let geometry: GeometryData = // ... obtained from VTK file parsing
///
/// // Create rendering mesh
/// let mesh = create_mesh_from_geometry(&geometry);
///
/// // Can be used directly in Bevy entities
/// commands.spawn((
///     Mesh3d(meshes.add(mesh)),
///     // ... other components
/// ));
/// ```
///
/// # Notes
///
/// - Input geometry data should be a valid triangle mesh
/// - Function prints color attribute application status
/// - Generated mesh uses triangle list topology
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
