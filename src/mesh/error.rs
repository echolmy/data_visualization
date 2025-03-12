use std::fmt;

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
