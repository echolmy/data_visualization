/// 颜色映射表模块
///
/// 提供从标量值到颜色的映射功能，支持多种预定义的颜色映射表

/// 颜色映射表
///
/// 提供从标量值到颜色的映射
#[derive(Debug, Clone)]
pub struct ColorMap {
    #[allow(dead_code)] // 保留用于调试和标识目的
    pub name: String,
    pub colors: Vec<[f32; 4]>,
}

impl ColorMap {
    /// 根据标量值获取颜色（离散映射）
    ///
    /// 参数:
    /// * `value` - 归一化的标量值 (0.0-1.0)
    ///
    /// 返回:
    /// * 对应的RGBA颜色
    #[allow(dead_code)] // 保留用于需要离散颜色映射的场景
    pub fn get_color(&self, value: f32) -> [f32; 4] {
        let normalized = value.clamp(0.0, 1.0);
        let index = (normalized * (self.colors.len() - 1) as f32).round() as usize;
        self.colors[index]
    }

    /// 根据标量值获取插值颜色（线性插值，推荐使用）
    ///
    /// 参数:
    /// * `value` - 归一化的标量值 (0.0-1.0)
    ///
    /// 返回:
    /// * 线性插值后的RGBA颜色
    pub fn get_interpolated_color(&self, value: f32) -> [f32; 4] {
        let normalized = value.clamp(0.0, 1.0);

        if self.colors.is_empty() {
            return [1.0, 1.0, 1.0, 1.0]; // 默认白色
        }

        if self.colors.len() == 1 {
            return self.colors[0];
        }

        // 计算浮点索引
        let float_index = normalized * (self.colors.len() - 1) as f32;
        let lower_index = float_index.floor() as usize;
        let upper_index = (lower_index + 1).min(self.colors.len() - 1);

        // 如果正好在边界上，直接返回
        if lower_index == upper_index {
            return self.colors[lower_index];
        }

        // 计算插值权重
        let weight = float_index - lower_index as f32;
        let lower_color = self.colors[lower_index];
        let upper_color = self.colors[upper_index];

        // 线性插值
        [
            lower_color[0] * (1.0 - weight) + upper_color[0] * weight,
            lower_color[1] * (1.0 - weight) + upper_color[1] * weight,
            lower_color[2] * (1.0 - weight) + upper_color[2] * weight,
            lower_color[3] * (1.0 - weight) + upper_color[3] * weight,
        ]
    }

    /// 双线性插值函数：考虑三角形内部的空间位置
    ///
    /// 使用重心坐标在三角形内部进行颜色插值，适用于高级可视化需求
    ///
    /// 参数:
    /// * `triangle_vertices` - 三角形三个顶点的位置 [[x1,y1,z1], [x2,y2,z2], [x3,y3,z3]]
    /// * `vertex_colors` - 三角形三个顶点的颜色 [[r1,g1,b1,a1], [r2,g2,b2,a2], [r3,g3,b3,a3]]
    /// * `point` - 三角形内部的查询点 [x,y,z]
    ///
    /// 返回:
    /// * 插值后的RGBA颜色
    #[allow(dead_code)] // 保留用于高级颜色插值功能
    pub fn bilinear_interpolate_color(
        triangle_vertices: &[[f32; 3]; 3],
        vertex_colors: &[[f32; 4]; 3],
        point: [f32; 3],
    ) -> [f32; 4] {
        // 计算重心坐标（barycentric coordinates）
        let v0 = [
            triangle_vertices[2][0] - triangle_vertices[0][0],
            triangle_vertices[2][1] - triangle_vertices[0][1],
            triangle_vertices[2][2] - triangle_vertices[0][2],
        ];
        let v1 = [
            triangle_vertices[1][0] - triangle_vertices[0][0],
            triangle_vertices[1][1] - triangle_vertices[0][1],
            triangle_vertices[1][2] - triangle_vertices[0][2],
        ];
        let v2 = [
            point[0] - triangle_vertices[0][0],
            point[1] - triangle_vertices[0][1],
            point[2] - triangle_vertices[0][2],
        ];

        // 计算点积
        let dot00 = v0[0] * v0[0] + v0[1] * v0[1] + v0[2] * v0[2];
        let dot01 = v0[0] * v1[0] + v0[1] * v1[1] + v0[2] * v1[2];
        let dot02 = v0[0] * v2[0] + v0[1] * v2[1] + v0[2] * v2[2];
        let dot11 = v1[0] * v1[0] + v1[1] * v1[1] + v1[2] * v1[2];
        let dot12 = v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2];

        // 计算重心坐标
        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        // 确保权重在有效范围内
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let w = (1.0 - u - v).clamp(0.0, 1.0);

        // 使用重心坐标进行颜色插值
        [
            vertex_colors[0][0] * w + vertex_colors[1][0] * v + vertex_colors[2][0] * u,
            vertex_colors[0][1] * w + vertex_colors[1][1] * v + vertex_colors[2][1] * u,
            vertex_colors[0][2] * w + vertex_colors[1][2] * v + vertex_colors[2][2] * u,
            vertex_colors[0][3] * w + vertex_colors[1][3] * v + vertex_colors[2][3] * u,
        ]
    }
}

/// 获取默认的颜色映射表
pub fn get_default_color_map() -> ColorMap {
    ColorMap {
        name: "default".to_string(),
        colors: vec![
            [0.0, 0.0, 0.6, 1.0], // 深海蓝
            [0.0, 0.0, 0.7, 1.0], // 更深蓝
            [0.0, 0.0, 0.8, 1.0], // 深蓝
            [0.0, 0.0, 0.9, 1.0], // 中深蓝
            [0.0, 0.0, 1.0, 1.0], // 蓝色
            [0.0, 0.1, 1.0, 1.0], // 蓝1
            [0.0, 0.2, 1.0, 1.0], // 蓝2
            [0.0, 0.3, 1.0, 1.0], // 蓝3
            [0.0, 0.4, 1.0, 1.0], // 蓝青
            [0.0, 0.5, 1.0, 1.0], // 蓝青2
            [0.0, 0.6, 1.0, 1.0], // 蓝青3
            [0.0, 0.7, 1.0, 1.0], // 蓝青4
            [0.0, 0.8, 1.0, 1.0], // 浅蓝青
            [0.0, 0.9, 1.0, 1.0], // 浅蓝青2
            [0.0, 1.0, 1.0, 1.0], // 青色
            [0.0, 1.0, 0.9, 1.0], // 青1
            [0.0, 1.0, 0.8, 1.0], // 青绿
            [0.0, 1.0, 0.7, 1.0], // 青绿2
            [0.0, 1.0, 0.6, 1.0], // 青绿3
            [0.0, 1.0, 0.5, 1.0], // 青绿4
            [0.0, 1.0, 0.4, 1.0], // 浅青绿
            [0.0, 1.0, 0.3, 1.0], // 浅青绿2
            [0.0, 1.0, 0.2, 1.0], // 浅青绿3
            [0.0, 1.0, 0.1, 1.0], // 浅青绿4
            [0.0, 1.0, 0.0, 1.0], // 绿色
            [0.2, 1.0, 0.0, 1.0], // 浅绿1
            [0.4, 1.0, 0.0, 1.0], // 浅绿
            [0.6, 1.0, 0.0, 1.0], // 黄绿1
            [0.8, 1.0, 0.0, 1.0], // 黄绿
            [1.0, 1.0, 0.0, 1.0], // 黄色
            [1.0, 0.9, 0.0, 1.0], // 橙黄1
            [1.0, 0.8, 0.0, 1.0], // 橙黄
            [1.0, 0.7, 0.0, 1.0], // 橙黄2
            [1.0, 0.6, 0.0, 1.0], // 橙
            [1.0, 0.5, 0.0, 1.0], // 橙2
            [1.0, 0.4, 0.0, 1.0], // 深橙
            [1.0, 0.3, 0.0, 1.0], // 深橙2
            [1.0, 0.2, 0.0, 1.0], // 红橙
            [1.0, 0.1, 0.0, 1.0], // 红橙2
            [1.0, 0.0, 0.0, 1.0], // 红色
        ],
    }
}

/// 获取彩虹色映射表
pub fn get_rainbow_color_map() -> ColorMap {
    ColorMap {
        name: "rainbow".to_string(),
        colors: vec![
            [0.6, 0.0, 1.0, 1.0], // 深紫
            [0.5, 0.0, 1.0, 1.0], // 紫色
            [0.4, 0.0, 1.0, 1.0], // 紫2
            [0.3, 0.0, 1.0, 1.0], // 深蓝紫
            [0.2, 0.0, 1.0, 1.0], // 蓝紫1
            [0.1, 0.0, 1.0, 1.0], // 蓝紫2
            [0.0, 0.0, 1.0, 1.0], // 蓝色
            [0.0, 0.1, 1.0, 1.0], // 蓝1
            [0.0, 0.2, 1.0, 1.0], // 蓝2
            [0.0, 0.3, 1.0, 1.0], // 蓝青
            [0.0, 0.4, 1.0, 1.0], // 蓝青2
            [0.0, 0.5, 1.0, 1.0], // 蓝青3
            [0.0, 0.6, 1.0, 1.0], // 蓝青4
            [0.0, 0.7, 1.0, 1.0], // 浅蓝青
            [0.0, 0.8, 1.0, 1.0], // 浅蓝青2
            [0.0, 0.9, 1.0, 1.0], // 浅蓝青3
            [0.0, 1.0, 1.0, 1.0], // 青色
            [0.0, 1.0, 0.9, 1.0], // 青1
            [0.0, 1.0, 0.8, 1.0], // 青2
            [0.0, 1.0, 0.7, 1.0], // 青绿
            [0.0, 1.0, 0.6, 1.0], // 青绿2
            [0.0, 1.0, 0.5, 1.0], // 青绿3
            [0.0, 1.0, 0.4, 1.0], // 青绿4
            [0.0, 1.0, 0.3, 1.0], // 浅青绿
            [0.0, 1.0, 0.2, 1.0], // 浅青绿2
            [0.0, 1.0, 0.1, 1.0], // 浅青绿3
            [0.0, 1.0, 0.0, 1.0], // 绿色
            [0.1, 1.0, 0.0, 1.0], // 浅绿1
            [0.2, 1.0, 0.0, 1.0], // 浅绿2
            [0.3, 1.0, 0.0, 1.0], // 浅绿
            [0.4, 1.0, 0.0, 1.0], // 浅绿3
            [0.5, 1.0, 0.0, 1.0], // 黄绿1
            [0.6, 1.0, 0.0, 1.0], // 黄绿2
            [0.7, 1.0, 0.0, 1.0], // 黄绿
            [0.8, 1.0, 0.0, 1.0], // 黄绿3
            [0.9, 1.0, 0.0, 1.0], // 黄绿4
            [1.0, 1.0, 0.0, 1.0], // 黄色
            [1.0, 0.9, 0.0, 1.0], // 橙黄1
            [1.0, 0.8, 0.0, 1.0], // 橙黄
            [1.0, 0.7, 0.0, 1.0], // 橙黄2
            [1.0, 0.6, 0.0, 1.0], // 橙1
            [1.0, 0.5, 0.0, 1.0], // 橙色
            [1.0, 0.4, 0.0, 1.0], // 橙2
            [1.0, 0.3, 0.0, 1.0], // 深橙
            [1.0, 0.2, 0.0, 1.0], // 深橙2
            [1.0, 0.1, 0.0, 1.0], // 红橙
            [1.0, 0.0, 0.0, 1.0], // 红色
        ],
    }
}

/// 获取热力图色映射表
pub fn get_hot_color_map() -> ColorMap {
    ColorMap {
        name: "hot".to_string(),
        colors: vec![
            [0.0, 0.0, 0.0, 1.0],  // 黑色
            [0.05, 0.0, 0.0, 1.0], // 极深红
            [0.1, 0.0, 0.0, 1.0],  // 超深红
            [0.15, 0.0, 0.0, 1.0], // 深暗红1
            [0.2, 0.0, 0.0, 1.0],  // 深暗红
            [0.25, 0.0, 0.0, 1.0], // 暗红1
            [0.3, 0.0, 0.0, 1.0],  // 暗红2
            [0.35, 0.0, 0.0, 1.0], // 暗红3
            [0.4, 0.0, 0.0, 1.0],  // 暗红
            [0.45, 0.0, 0.0, 1.0], // 暗红4
            [0.5, 0.0, 0.0, 1.0],  // 中暗红
            [0.55, 0.0, 0.0, 1.0], // 中暗红2
            [0.6, 0.0, 0.0, 1.0],  // 深红色
            [0.65, 0.0, 0.0, 1.0], // 深红2
            [0.7, 0.0, 0.0, 1.0],  // 深红3
            [0.75, 0.0, 0.0, 1.0], // 中红1
            [0.8, 0.0, 0.0, 1.0],  // 中红
            [0.85, 0.0, 0.0, 1.0], // 中红2
            [0.9, 0.0, 0.0, 1.0],  // 中红3
            [0.95, 0.0, 0.0, 1.0], // 亮红1
            [1.0, 0.0, 0.0, 1.0],  // 红色
            [1.0, 0.05, 0.0, 1.0], // 红橙1
            [1.0, 0.1, 0.0, 1.0],  // 红橙2
            [1.0, 0.15, 0.0, 1.0], // 红橙3
            [1.0, 0.2, 0.0, 1.0],  // 红橙
            [1.0, 0.25, 0.0, 1.0], // 红橙4
            [1.0, 0.3, 0.0, 1.0],  // 深橙红
            [1.0, 0.35, 0.0, 1.0], // 深橙红2
            [1.0, 0.4, 0.0, 1.0],  // 深橙
            [1.0, 0.45, 0.0, 1.0], // 深橙2
            [1.0, 0.5, 0.0, 1.0],  // 橙色1
            [1.0, 0.55, 0.0, 1.0], // 橙色2
            [1.0, 0.6, 0.0, 1.0],  // 橙色
            [1.0, 0.65, 0.0, 1.0], // 橙色3
            [1.0, 0.7, 0.0, 1.0],  // 浅橙1
            [1.0, 0.75, 0.0, 1.0], // 浅橙2
            [1.0, 0.8, 0.0, 1.0],  // 浅橙
            [1.0, 0.85, 0.0, 1.0], // 橙黄1
            [1.0, 0.9, 0.0, 1.0],  // 橙黄2
            [1.0, 0.95, 0.0, 1.0], // 橙黄3
            [1.0, 1.0, 0.0, 1.0],  // 黄色
            [1.0, 1.0, 0.05, 1.0], // 浅黄1
            [1.0, 1.0, 0.1, 1.0],  // 浅黄2
            [1.0, 1.0, 0.15, 1.0], // 浅黄3
            [1.0, 1.0, 0.2, 1.0],  // 浅黄
            [1.0, 1.0, 0.25, 1.0], // 浅黄4
            [1.0, 1.0, 0.3, 1.0],  // 中浅黄1
            [1.0, 1.0, 0.35, 1.0], // 中浅黄2
            [1.0, 1.0, 0.4, 1.0],  // 中浅黄
            [1.0, 1.0, 0.45, 1.0], // 中浅黄3
            [1.0, 1.0, 0.5, 1.0],  // 中浅黄4
            [1.0, 1.0, 0.55, 1.0], // 亮黄1
            [1.0, 1.0, 0.6, 1.0],  // 亮黄
            [1.0, 1.0, 0.65, 1.0], // 亮黄2
            [1.0, 1.0, 0.7, 1.0],  // 很亮黄1
            [1.0, 1.0, 0.75, 1.0], // 很亮黄2
            [1.0, 1.0, 0.8, 1.0],  // 很亮黄
            [1.0, 1.0, 0.85, 1.0], // 很亮黄3
            [1.0, 1.0, 0.9, 1.0],  // 极亮黄
            [1.0, 1.0, 0.95, 1.0], // 接近白
            [1.0, 1.0, 1.0, 1.0],  // 白色
        ],
    }
}

/// 获取高分辨率彩虹色映射表（更多采样点）
pub fn get_high_res_rainbow_color_map() -> ColorMap {
    ColorMap {
        name: "high_res_rainbow".to_string(),
        colors: vec![
            [0.6, 0.0, 1.0, 1.0],  // 深紫
            [0.55, 0.0, 1.0, 1.0], // 深紫2
            [0.5, 0.0, 1.0, 1.0],  // 紫色
            [0.45, 0.0, 1.0, 1.0], // 紫2
            [0.4, 0.0, 1.0, 1.0],  // 紫蓝1
            [0.35, 0.0, 1.0, 1.0], // 紫蓝2
            [0.3, 0.0, 1.0, 1.0],  // 深蓝紫
            [0.25, 0.0, 1.0, 1.0], // 深蓝紫2
            [0.2, 0.0, 1.0, 1.0],  // 蓝紫1
            [0.15, 0.0, 1.0, 1.0], // 蓝紫2
            [0.1, 0.0, 1.0, 1.0],  // 蓝紫3
            [0.05, 0.0, 1.0, 1.0], // 蓝紫4
            [0.0, 0.0, 1.0, 1.0],  // 蓝色
            [0.0, 0.05, 1.0, 1.0], // 蓝1
            [0.0, 0.1, 1.0, 1.0],  // 蓝2
            [0.0, 0.15, 1.0, 1.0], // 蓝3
            [0.0, 0.2, 1.0, 1.0],  // 蓝青1
            [0.0, 0.25, 1.0, 1.0], // 蓝青2
            [0.0, 0.3, 1.0, 1.0],  // 蓝青
            [0.0, 0.35, 1.0, 1.0], // 蓝青3
            [0.0, 0.4, 1.0, 1.0],  // 蓝青4
            [0.0, 0.45, 1.0, 1.0], // 蓝青5
            [0.0, 0.5, 1.0, 1.0],  // 中蓝青
            [0.0, 0.55, 1.0, 1.0], // 中蓝青2
            [0.0, 0.6, 1.0, 1.0],  // 浅蓝青1
            [0.0, 0.65, 1.0, 1.0], // 浅蓝青2
            [0.0, 0.7, 1.0, 1.0],  // 浅蓝青
            [0.0, 0.75, 1.0, 1.0], // 浅蓝青3
            [0.0, 0.8, 1.0, 1.0],  // 浅蓝青4
            [0.0, 0.85, 1.0, 1.0], // 浅蓝青5
            [0.0, 0.9, 1.0, 1.0],  // 青蓝1
            [0.0, 0.95, 1.0, 1.0], // 青蓝2
            [0.0, 1.0, 1.0, 1.0],  // 青色
            [0.0, 1.0, 0.95, 1.0], // 青1
            [0.0, 1.0, 0.9, 1.0],  // 青2
            [0.0, 1.0, 0.85, 1.0], // 青绿1
            [0.0, 1.0, 0.8, 1.0],  // 青绿2
            [0.0, 1.0, 0.75, 1.0], // 青绿3
            [0.0, 1.0, 0.7, 1.0],  // 青绿
            [0.0, 1.0, 0.65, 1.0], // 青绿4
            [0.0, 1.0, 0.6, 1.0],  // 青绿5
            [0.0, 1.0, 0.55, 1.0], // 中青绿1
            [0.0, 1.0, 0.5, 1.0],  // 中青绿
            [0.0, 1.0, 0.45, 1.0], // 中青绿2
            [0.0, 1.0, 0.4, 1.0],  // 浅青绿1
            [0.0, 1.0, 0.35, 1.0], // 浅青绿2
            [0.0, 1.0, 0.3, 1.0],  // 浅青绿
            [0.0, 1.0, 0.25, 1.0], // 浅青绿3
            [0.0, 1.0, 0.2, 1.0],  // 浅青绿4
            [0.0, 1.0, 0.15, 1.0], // 绿青1
            [0.0, 1.0, 0.1, 1.0],  // 绿青2
            [0.0, 1.0, 0.05, 1.0], // 绿青3
            [0.0, 1.0, 0.0, 1.0],  // 绿色
            [0.05, 1.0, 0.0, 1.0], // 绿1
            [0.1, 1.0, 0.0, 1.0],  // 绿2
            [0.15, 1.0, 0.0, 1.0], // 浅绿1
            [0.2, 1.0, 0.0, 1.0],  // 浅绿2
            [0.25, 1.0, 0.0, 1.0], // 浅绿3
            [0.3, 1.0, 0.0, 1.0],  // 浅绿
            [0.35, 1.0, 0.0, 1.0], // 浅绿4
            [0.4, 1.0, 0.0, 1.0],  // 黄绿1
            [0.45, 1.0, 0.0, 1.0], // 黄绿2
            [0.5, 1.0, 0.0, 1.0],  // 黄绿3
            [0.55, 1.0, 0.0, 1.0], // 黄绿4
            [0.6, 1.0, 0.0, 1.0],  // 黄绿5
            [0.65, 1.0, 0.0, 1.0], // 黄绿6
            [0.7, 1.0, 0.0, 1.0],  // 黄绿
            [0.75, 1.0, 0.0, 1.0], // 黄绿7
            [0.8, 1.0, 0.0, 1.0],  // 绿黄1
            [0.85, 1.0, 0.0, 1.0], // 绿黄2
            [0.9, 1.0, 0.0, 1.0],  // 绿黄3
            [0.95, 1.0, 0.0, 1.0], // 绿黄4
            [1.0, 1.0, 0.0, 1.0],  // 黄色
            [1.0, 0.95, 0.0, 1.0], // 黄1
            [1.0, 0.9, 0.0, 1.0],  // 黄橙1
            [1.0, 0.85, 0.0, 1.0], // 黄橙2
            [1.0, 0.8, 0.0, 1.0],  // 橙黄
            [1.0, 0.75, 0.0, 1.0], // 橙黄2
            [1.0, 0.7, 0.0, 1.0],  // 橙黄3
            [1.0, 0.65, 0.0, 1.0], // 橙色1
            [1.0, 0.6, 0.0, 1.0],  // 橙色2
            [1.0, 0.55, 0.0, 1.0], // 橙色3
            [1.0, 0.5, 0.0, 1.0],  // 橙色
            [1.0, 0.45, 0.0, 1.0], // 橙色4
            [1.0, 0.4, 0.0, 1.0],  // 深橙1
            [1.0, 0.35, 0.0, 1.0], // 深橙2
            [1.0, 0.3, 0.0, 1.0],  // 深橙
            [1.0, 0.25, 0.0, 1.0], // 深橙3
            [1.0, 0.2, 0.0, 1.0],  // 橙红1
            [1.0, 0.15, 0.0, 1.0], // 橙红2
            [1.0, 0.1, 0.0, 1.0],  // 橙红3
            [1.0, 0.05, 0.0, 1.0], // 橙红4
            [1.0, 0.0, 0.0, 1.0],  // 红色
        ],
    }
}

/// 获取科学可视化常用的Viridis色映射表
pub fn get_viridis_color_map() -> ColorMap {
    ColorMap {
        name: "viridis".to_string(),
        colors: vec![
            [0.267004, 0.004874, 0.329415, 1.0], // 深紫
            [0.275191, 0.060826, 0.390374, 1.0], // 紫
            [0.282623, 0.140926, 0.457517, 1.0], // 紫色
            [0.285109, 0.195242, 0.495702, 1.0], // 蓝紫
            [0.253935, 0.265254, 0.529983, 1.0], // 蓝紫
            [0.230341, 0.318626, 0.545695, 1.0], // 蓝
            [0.206756, 0.371758, 0.553117, 1.0], // 蓝色
            [0.184586, 0.423943, 0.556295, 1.0], // 青蓝
            [0.163625, 0.471133, 0.558148, 1.0], // 青蓝
            [0.144544, 0.516775, 0.557885, 1.0], // 青
            [0.127568, 0.566949, 0.550556, 1.0], // 青色
            [0.131109, 0.616355, 0.533488, 1.0], // 青绿
            [0.134692, 0.658636, 0.517649, 1.0], // 青绿
            [0.177423, 0.699873, 0.490448, 1.0], // 绿青
            [0.266941, 0.748751, 0.440573, 1.0], // 绿色
            [0.369214, 0.788888, 0.382914, 1.0], // 黄绿
            [0.477504, 0.821444, 0.318195, 1.0], // 黄绿
            [0.590330, 0.851556, 0.248701, 1.0], // 绿黄
            [0.706680, 0.877588, 0.175630, 1.0], // 黄
            [0.741388, 0.873449, 0.149561, 1.0], // 黄色
            [0.865006, 0.897915, 0.145833, 1.0], // 亮黄
            [0.993248, 0.906157, 0.143936, 1.0], // 亮黄
        ],
    }
}

/// 获取冷色调颜色映射表（蓝色到青色系列）
pub fn get_cool_color_map() -> ColorMap {
    ColorMap {
        name: "cool".to_string(),
        colors: vec![
            [0.0, 0.0, 0.2, 1.0],  // 极深海军蓝
            [0.0, 0.0, 0.25, 1.0], // 深海军蓝1
            [0.0, 0.0, 0.3, 1.0],  // 深海军蓝
            [0.0, 0.0, 0.35, 1.0], // 深海军蓝2
            [0.0, 0.0, 0.4, 1.0],  // 深蓝1
            [0.0, 0.0, 0.45, 1.0], // 深蓝2
            [0.0, 0.0, 0.5, 1.0],  // 深蓝
            [0.0, 0.0, 0.55, 1.0], // 深蓝3
            [0.0, 0.0, 0.6, 1.0],  // 中深蓝
            [0.0, 0.0, 0.65, 1.0], // 中深蓝2
            [0.0, 0.0, 0.7, 1.0],  // 中蓝
            [0.0, 0.0, 0.75, 1.0], // 中蓝2
            [0.0, 0.0, 0.8, 1.0],  // 中蓝3
            [0.0, 0.0, 0.85, 1.0], // 中蓝4
            [0.0, 0.0, 0.9, 1.0],  // 亮蓝1
            [0.0, 0.0, 0.95, 1.0], // 亮蓝2
            [0.0, 0.0, 1.0, 1.0],  // 蓝色
            [0.0, 0.05, 1.0, 1.0], // 蓝紫1
            [0.0, 0.1, 1.0, 1.0],  // 蓝紫2
            [0.0, 0.15, 1.0, 1.0], // 蓝紫3
            [0.0, 0.2, 1.0, 1.0],  // 蓝紫
            [0.0, 0.25, 1.0, 1.0], // 蓝紫4
            [0.0, 0.3, 1.0, 1.0],  // 浅蓝1
            [0.0, 0.35, 1.0, 1.0], // 浅蓝2
            [0.0, 0.4, 1.0, 1.0],  // 浅蓝
            [0.0, 0.45, 1.0, 1.0], // 浅蓝3
            [0.0, 0.5, 1.0, 1.0],  // 浅蓝4
            [0.0, 0.55, 1.0, 1.0], // 天蓝1
            [0.0, 0.6, 1.0, 1.0],  // 天蓝
            [0.0, 0.65, 1.0, 1.0], // 天蓝2
            [0.0, 0.7, 1.0, 1.0],  // 天蓝3
            [0.0, 0.75, 1.0, 1.0], // 淡蓝1
            [0.0, 0.8, 1.0, 1.0],  // 淡蓝
            [0.0, 0.85, 1.0, 1.0], // 淡蓝2
            [0.0, 0.9, 1.0, 1.0],  // 淡蓝3
            [0.0, 0.95, 1.0, 1.0], // 淡蓝4
            [0.0, 1.0, 1.0, 1.0],  // 青色
            [0.05, 1.0, 1.0, 1.0], // 浅青1
            [0.1, 1.0, 1.0, 1.0],  // 浅青2
            [0.15, 1.0, 1.0, 1.0], // 浅青3
            [0.2, 1.0, 1.0, 1.0],  // 浅青
            [0.25, 1.0, 1.0, 1.0], // 浅青4
            [0.3, 1.0, 1.0, 1.0],  // 淡青1
            [0.35, 1.0, 1.0, 1.0], // 淡青2
            [0.4, 1.0, 1.0, 1.0],  // 淡青
            [0.45, 1.0, 1.0, 1.0], // 淡青3
            [0.5, 1.0, 1.0, 1.0],  // 淡青4
            [0.55, 1.0, 1.0, 1.0], // 很淡青1
            [0.6, 1.0, 1.0, 1.0],  // 很淡青
            [0.65, 1.0, 1.0, 1.0], // 很淡青2
            [0.7, 1.0, 1.0, 1.0],  // 很淡青3
            [0.75, 1.0, 1.0, 1.0], // 极淡青1
            [0.8, 1.0, 1.0, 1.0],  // 极淡青
            [0.85, 1.0, 1.0, 1.0], // 极淡青2
            [0.9, 1.0, 1.0, 1.0],  // 极淡青3
            [0.95, 1.0, 1.0, 1.0], // 接近白1
            [1.0, 1.0, 1.0, 1.0],  // 白色
        ],
    }
}

/// 获取暖色调颜色映射表（红色到黄色系列）
pub fn get_warm_color_map() -> ColorMap {
    ColorMap {
        name: "warm".to_string(),
        colors: vec![
            [0.2, 0.0, 0.0, 1.0],   // 极深红
            [0.25, 0.0, 0.0, 1.0],  // 深红1
            [0.3, 0.0, 0.0, 1.0],   // 深红
            [0.35, 0.0, 0.0, 1.0],  // 深红2
            [0.4, 0.0, 0.0, 1.0],   // 深红3
            [0.45, 0.0, 0.0, 1.0],  // 中深红1
            [0.5, 0.0, 0.0, 1.0],   // 暗红
            [0.55, 0.0, 0.0, 1.0],  // 中深红2
            [0.6, 0.0, 0.0, 1.0],   // 中红1
            [0.65, 0.0, 0.0, 1.0],  // 中红
            [0.7, 0.0, 0.0, 1.0],   // 中红
            [0.75, 0.0, 0.0, 1.0],  // 中红3
            [0.8, 0.0, 0.0, 1.0],   // 红1
            [0.85, 0.0, 0.0, 1.0],  // 红2
            [0.9, 0.0, 0.0, 1.0],   // 红3
            [0.95, 0.0, 0.0, 1.0],  // 红4
            [1.0, 0.0, 0.0, 1.0],   // 红色
            [1.0, 0.025, 0.0, 1.0], // 红橙1
            [1.0, 0.05, 0.0, 1.0],  // 红橙2
            [1.0, 0.075, 0.0, 1.0], // 红橙3
            [1.0, 0.1, 0.0, 1.0],   // 红橙
            [1.0, 0.125, 0.0, 1.0], // 红橙4
            [1.0, 0.15, 0.0, 1.0],  // 深橙红1
            [1.0, 0.175, 0.0, 1.0], // 深橙红2
            [1.0, 0.2, 0.0, 1.0],   // 深橙红
            [1.0, 0.225, 0.0, 1.0], // 深橙红3
            [1.0, 0.25, 0.0, 1.0],  // 橙红1
            [1.0, 0.275, 0.0, 1.0], // 橙红2
            [1.0, 0.3, 0.0, 1.0],   // 橙红
            [1.0, 0.325, 0.0, 1.0], // 橙红3
            [1.0, 0.35, 0.0, 1.0],  // 深橙1
            [1.0, 0.375, 0.0, 1.0], // 深橙2
            [1.0, 0.4, 0.0, 1.0],   // 深橙
            [1.0, 0.425, 0.0, 1.0], // 深橙3
            [1.0, 0.45, 0.0, 1.0],  // 橙色1
            [1.0, 0.475, 0.0, 1.0], // 橙色2
            [1.0, 0.5, 0.0, 1.0],   // 橙色
            [1.0, 0.525, 0.0, 1.0], // 橙色3
            [1.0, 0.55, 0.0, 1.0],  // 浅橙1
            [1.0, 0.575, 0.0, 1.0], // 浅橙2
            [1.0, 0.6, 0.0, 1.0],   // 浅橙
            [1.0, 0.625, 0.0, 1.0], // 浅橙3
            [1.0, 0.65, 0.0, 1.0],  // 橙黄1
            [1.0, 0.675, 0.0, 1.0], // 橙黄2
            [1.0, 0.7, 0.0, 1.0],   // 橙黄
            [1.0, 0.725, 0.0, 1.0], // 橙黄3
            [1.0, 0.75, 0.0, 1.0],  // 深黄1
            [1.0, 0.775, 0.0, 1.0], // 深黄2
            [1.0, 0.8, 0.0, 1.0],   // 深黄
            [1.0, 0.825, 0.0, 1.0], // 深黄3
            [1.0, 0.85, 0.0, 1.0],  // 黄色1
            [1.0, 0.875, 0.0, 1.0], // 黄色2
            [1.0, 0.9, 0.0, 1.0],   // 黄色
            [1.0, 0.925, 0.0, 1.0], // 黄色3
            [1.0, 0.95, 0.0, 1.0],  // 黄色4
            [1.0, 0.975, 0.0, 1.0], // 纯黄1
            [1.0, 1.0, 0.0, 1.0],   // 纯黄
            [1.0, 1.0, 0.1, 1.0],   // 浅黄1
            [1.0, 1.0, 0.2, 1.0],   // 浅黄
            [1.0, 1.0, 0.3, 1.0],   // 浅黄2
            [1.0, 1.0, 0.4, 1.0],   // 淡黄1
            [1.0, 1.0, 0.5, 1.0],   // 淡黄
            [1.0, 1.0, 0.6, 1.0],   // 淡黄2
            [1.0, 1.0, 0.7, 1.0],   // 很淡黄1
            [1.0, 1.0, 0.8, 1.0],   // 很淡黄
            [1.0, 1.0, 0.85, 1.0],  // 很淡黄2
            [1.0, 1.0, 0.9, 1.0],   // 极淡黄1
            [1.0, 1.0, 0.95, 1.0],  // 极淡黄2
            [1.0, 1.0, 1.0, 1.0],   // 白色
        ],
    }
}

/// 获取指定名称的颜色映射表
pub fn get_color_map(name: &str) -> ColorMap {
    match name {
        "rainbow" => get_rainbow_color_map(),
        "high_res_rainbow" => get_high_res_rainbow_color_map(),
        "viridis" => get_viridis_color_map(),
        "hot" => get_hot_color_map(),
        "cool" => get_cool_color_map(),
        "warm" => get_warm_color_map(),
        _ => get_default_color_map(),
    }
}
