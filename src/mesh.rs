use std::fmt;
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
            VtkError::LoadError(msg) => write!(f, "加载VTK文件错误: {}", msg),
            VtkError::InvalidFormat(detail) => write!(f, "VTK格式无效: {}", detail),
            VtkError::UnsupportedDataType => write!(f, "不支持的数据类型"),
            VtkError::MissingData(what) => write!(f, "缺少数据: {}", what),
            VtkError::IndexOutOfBounds { index, max } => {
                write!(f, "索引超出边界: {} (最大值为 {})", index, max)
            }
            VtkError::DataTypeMismatch { expected, found } => {
                write!(f, "数据类型不匹配: 期望 {}, 找到 {}", expected, found)
            }
            VtkError::AttributeMismatch {
                attribute_size,
                expected_size,
            } => {
                write!(
                    f,
                    "属性大小不匹配: 属性大小 {}, 期望 {}",
                    attribute_size, expected_size
                )
            }
            VtkError::ConversionError(msg) => write!(f, "转换错误: {}", msg),
            VtkError::IoError(err) => write!(f, "IO错误: {}", err),
            VtkError::GenericError(msg) => write!(f, "错误: {}", msg),
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

    println!("提取的几何数据属性信息: {:?}", &geometry.attributes);

    // 创建带属性的网格
    let mut mesh = create_mesh_legacy(geometry.clone());

    // 应用颜色属性
    let _ = geometry.apply_cell_color_scalars(&mut mesh);

    // 如果没有单元格颜色，尝试应用点颜色
    if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
        let _ = geometry.apply_point_color_scalars(&mut mesh);
    }

    // 应用其他标量属性（如果有）
    let _ = geometry.apply_scalar_attributes(&mut mesh);

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

//**************************************************************************//
