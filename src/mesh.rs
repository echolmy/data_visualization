use std::fmt;
pub mod color_maps;
pub mod triangulation;
pub mod vtk;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::*;

use self::vtk::*;

#[derive(Debug)]
#[allow(dead_code)]
pub enum VtkError {
    LoadError(String),
    InvalidFormat(&'static str),
    UnsupportedDataType,
    MissingData(&'static str),
    IndexOutOfBounds {
        index: usize,
        max: usize,
    },
    DataTypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    AttributeMismatch {
        attribute_size: usize,
        expected_size: usize,
    },
    ConversionError(String),
    IoError(std::io::Error),
    GenericError(String),
}

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

impl std::error::Error for VtkError {}

impl From<std::io::Error> for VtkError {
    fn from(err: std::io::Error) -> Self {
        VtkError::IoError(err)
    }
}

//************************************* Main Process Logic**************************************//
/// Process a legacy VTK file and create a mesh with attributes
pub fn process_vtk_file_legacy(path: &PathBuf) -> Result<Mesh, VtkError> {
    let geometry: GeometryData;
    let vtk = Vtk::import(PathBuf::from(format!("{}", path.to_string_lossy())))
        .map_err(|e| VtkError::LoadError(e.to_string()))?;

    // 打印VTK文件的基本信息
    print_vtk_info(&vtk);

    match vtk.data {
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            let extractor = UnstructuredGridExtractor;
            geometry = extractor.process_legacy(pieces)?;
        }
        model::DataSet::PolyData { meta: _, pieces } => {
            let extractor = PolyDataExtractor;
            geometry = extractor.process_legacy(pieces)?;
        }
        _ => {
            return Err(VtkError::UnsupportedDataType);
        }
    }

    println!(
        "Extracted geometry data attributes: {:?}",
        &geometry.attributes
    );

    // 打印几何数据的基本信息
    print_geometry_info(&geometry);

    // 创建基本网格
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // 添加顶点位置
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(geometry.vertices.clone()),
    );

    // 添加顶点索引
    mesh.insert_indices(Indices::U32(geometry.indices.clone()));

    // 计算法线
    mesh.compute_normals();

    // 按优先级应用颜色属性
    // 1. 首先尝试应用标量属性（通常是最重要的数据）
    let scalar_applied = geometry.apply_scalar_attributes(&mut mesh).is_ok();
    println!("Scalar attributes applied: {}", scalar_applied);

    // 2. 如果没有标量属性，尝试应用单元格颜色
    if !scalar_applied {
        let cell_color_applied = geometry.apply_cell_color_scalars(&mut mesh).is_ok();
        println!("Cell color attributes applied: {}", cell_color_applied);

        // 3. 如果没有单元格颜色，尝试应用点颜色
        if !cell_color_applied {
            let point_color_applied = geometry.apply_point_color_scalars(&mut mesh).is_ok();
            println!("Point color attributes applied: {}", point_color_applied);

            // 4. 如果没有任何颜色属性，应用默认颜色
            if !point_color_applied {
                println!("No color attributes found, applying default colors");
                // 默认使用白色
                let default_colors = vec![[1.0, 1.0, 1.0, 1.0]; geometry.vertices.len()];
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    VertexAttributeValues::from(default_colors),
                );
            }
        }
    }

    Ok(mesh)
}

/// Create a mesh from geometry data
pub fn create_mesh_legacy(geometry: GeometryData) -> Mesh {
    // initialize a mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    // Set color
    println!("{:?}", &geometry.attributes);
    let _ = geometry.apply_cell_color_scalars(&mut mesh);

    // process vertices position attributes
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(geometry.vertices),
    );

    // process vertices indices attributes
    mesh.insert_indices(Indices::U32(geometry.indices));

    // compute normals
    mesh.compute_normals();

    mesh
}

/// 打印VTK文件的基本信息
fn print_vtk_info(vtk: &Vtk) {
    println!("VTK文件信息:");
    println!("  版本: {:?}", vtk.version);
    println!("  标题: {}", vtk.title);

    match &vtk.data {
        model::DataSet::UnstructuredGrid { meta, pieces } => {
            println!("  数据类型: UnstructuredGrid");
            println!("  元数据: {:?}", meta);
            println!("  片段数量: {}", pieces.len());
        }
        model::DataSet::PolyData { meta, pieces } => {
            println!("  数据类型: PolyData");
            println!("  元数据: {:?}", meta);
            println!("  片段数量: {}", pieces.len());
        }
        _ => println!("  数据类型: 其他"),
    }
}

/// 打印几何数据的基本信息
fn print_geometry_info(geometry: &GeometryData) {
    println!("几何数据信息:");
    println!("  顶点数量: {}", geometry.vertices.len());
    println!("  索引数量: {}", geometry.indices.len());
    println!("  三角形数量: {}", geometry.indices.len() / 3);

    if let Some(attributes) = &geometry.attributes {
        println!("  属性数量: {}", attributes.len());

        for ((name, location), attr) in attributes.iter() {
            match attr {
                AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                    lookup_table,
                } => {
                    println!("  标量属性: {} (位置: {:?})", name, location);
                    println!("    组件数量: {}", num_comp);
                    println!("    查找表名称: {}", table_name);
                    println!("    数据长度: {}", data.len());
                    if let Some(lut) = lookup_table {
                        println!("    查找表颜色数量: {}", lut.len());
                    }
                }
                AttributeType::ColorScalar { nvalues, data } => {
                    println!("  颜色标量属性: {} (位置: {:?})", name, location);
                    println!("    值数量: {}", nvalues);
                    println!("    数据长度: {}", data.len());
                }
                AttributeType::Vector(data) => {
                    println!("  向量属性: {} (位置: {:?})", name, location);
                    println!("    数据长度: {}", data.len());
                }
            }
        }
    } else {
        println!("  没有属性");
    }

    println!("  查找表数量: {}", geometry.lookup_tables.len());
    for (name, colors) in &geometry.lookup_tables {
        println!("    查找表: {} (颜色数量: {})", name, colors.len());
        if !colors.is_empty() {
            println!(
                "      第一个颜色: [{:.2}, {:.2}, {:.2}, {:.2}]",
                colors[0][0], colors[0][1], colors[0][2], colors[0][3]
            );
            println!(
                "      最后一个颜色: [{:.2}, {:.2}, {:.2}, {:.2}]",
                colors[colors.len() - 1][0],
                colors[colors.len() - 1][1],
                colors[colors.len() - 1][2],
                colors[colors.len() - 1][3]
            );
        }
    }
}

//**************************************************************************//
