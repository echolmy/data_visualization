/// 颜色映射表模块
///
/// 提供从标量值到颜色的映射功能，支持多种预定义的颜色映射表

/// 颜色映射表
///
/// 提供从标量值到颜色的映射
#[derive(Debug, Clone)]
pub struct ColorMap {
    pub name: String,
    pub colors: Vec<[f32; 4]>,
}

impl ColorMap {
    /// 根据标量值获取颜色
    ///
    /// 参数:
    /// * `value` - 归一化的标量值 (0.0-1.0)
    ///
    /// 返回:
    /// * 对应的RGBA颜色
    pub fn get_color(&self, value: f32) -> [f32; 4] {
        let normalized = value.clamp(0.0, 1.0);
        let index = (normalized * (self.colors.len() - 1) as f32).round() as usize;
        self.colors[index]
    }
}

/// 获取默认的颜色映射表
pub fn get_default_color_map() -> ColorMap {
    ColorMap {
        name: "default".to_string(),
        colors: vec![
            [0.0, 0.0, 1.0, 1.0], // 蓝色
            [0.0, 1.0, 1.0, 1.0], // 青色
            [0.0, 1.0, 0.0, 1.0], // 绿色
            [1.0, 1.0, 0.0, 1.0], // 黄色
            [1.0, 0.0, 0.0, 1.0], // 红色
        ],
    }
}

/// 获取彩虹色映射表
pub fn get_rainbow_color_map() -> ColorMap {
    ColorMap {
        name: "rainbow".to_string(),
        colors: vec![
            [0.5, 0.0, 1.0, 1.0], // 紫色
            [0.0, 0.0, 1.0, 1.0], // 蓝色
            [0.0, 1.0, 1.0, 1.0], // 青色
            [0.0, 1.0, 0.0, 1.0], // 绿色
            [1.0, 1.0, 0.0, 1.0], // 黄色
            [1.0, 0.5, 0.0, 1.0], // 橙色
            [1.0, 0.0, 0.0, 1.0], // 红色
        ],
    }
}

/// 获取热力图色映射表
pub fn get_hot_color_map() -> ColorMap {
    ColorMap {
        name: "hot".to_string(),
        colors: vec![
            [0.0, 0.0, 0.0, 1.0], // 黑色
            [0.5, 0.0, 0.0, 1.0], // 深红色
            [1.0, 0.0, 0.0, 1.0], // 红色
            [1.0, 0.5, 0.0, 1.0], // 橙色
            [1.0, 1.0, 0.0, 1.0], // 黄色
            [1.0, 1.0, 1.0, 1.0], // 白色
        ],
    }
}

/// 获取指定名称的颜色映射表
pub fn get_color_map(name: &str) -> ColorMap {
    match name {
        "rainbow" => get_rainbow_color_map(),
        "hot" => get_hot_color_map(),
        _ => get_default_color_map(),
    }
}
