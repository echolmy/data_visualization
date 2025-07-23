use crate::mesh;
use crate::mesh::color_maps::{get_color_map, ColorMap};
use bevy::prelude::*;
use bevy_egui::*;

/// 颜色条配置资源
///
/// 管理颜色条的显示状态、数值范围和样式设置
#[derive(Resource)]
pub struct ColorBarConfig {
    /// 是否显示颜色条
    pub visible: bool,
    /// 当前使用的颜色映射表名称
    pub color_map_name: String,
    /// 数值范围的最小值
    pub min_value: f32,
    /// 数值范围的最大值
    pub max_value: f32,
    /// 颜色条标题
    pub title: String,
    /// 数值单位
    pub unit: String,
    /// 标记配置是否发生变化，用于触发网格重新着色
    pub has_changed: bool,
}

impl Default for ColorBarConfig {
    fn default() -> Self {
        Self {
            visible: true,
            color_map_name: "rainbow".to_string(),
            min_value: 0.0,
            max_value: 1.0,
            title: "value".to_string(),
            unit: "".to_string(),
            has_changed: false,
        }
    }
}

/// 渲染颜色条UI
///
/// 在界面右侧显示一个颜色条面板，显示当前颜色映射表和数值范围
/// 内联渲染颜色条（从initialize_ui_systems调用）
/// 确保在TopBottomPanel之后立即显示SidePanel，避免布局冲突
pub fn render_color_bar_inline(
    contexts: &mut EguiContexts,
    mut color_bar_config: ResMut<ColorBarConfig>,
) {
    // 使用 SidePanel，但设置正确的属性避免跳动
    egui::SidePanel::right("color_bar_panel")
        .min_width(180.0) // 固定最小宽度
        .max_width(180.0) // 固定最大宽度，防止调整大小
        .default_width(180.0) // 固定默认宽度
        .resizable(false) // 禁止用户调整大小
        .show_separator_line(false) // 隐藏分隔线减少视觉干扰
        .show(contexts.ctx_mut(), |ui| {
            // 使用简单的垂直布局，不再使用复杂的绝对定位
            ui.vertical(|ui| {
                // ui.heading("color bar");

                ui.separator();

                // 颜色映射表选择
                ui.label("Color Map:");
                egui::ComboBox::from_id_salt("color_map")
                    .selected_text(&color_bar_config.color_map_name)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        let color_maps =
                            ["default", "rainbow", "high_res_rainbow", "viridis", "hot"];
                        for &color_map in &color_maps {
                            let value = ui.selectable_value(
                                &mut color_bar_config.color_map_name,
                                color_map.to_string(),
                                color_map,
                            );
                            if value.changed() {
                                color_bar_config.has_changed = true;
                            }
                        }
                    });

                ui.separator();

                // 数值范围控制
                ui.label("Value Range:");

                ui.horizontal(|ui| {
                    ui.label("Min:");
                    let min_response = ui.add_sized(
                        [80.0, 20.0],
                        egui::DragValue::new(&mut color_bar_config.min_value).speed(0.1),
                    );
                    if min_response.changed() {
                        color_bar_config.has_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Max:");
                    let max_response = ui.add_sized(
                        [80.0, 20.0],
                        egui::DragValue::new(&mut color_bar_config.max_value).speed(0.1),
                    );
                    if max_response.changed() {
                        color_bar_config.has_changed = true;
                    }
                });

                ui.separator();

                // 获取当前颜色映射表并渲染颜色条
                let color_map = get_color_map(&color_bar_config.color_map_name);
                render_color_gradient_simple(ui, &color_map, &color_bar_config);

                ui.separator();

                // 标签设置
                ui.label("Label Settings:");

                ui.horizontal(|ui| {
                    ui.label("Title:");
                    let title_response = ui.add_sized(
                        [80.0, 20.0],
                        egui::TextEdit::singleline(&mut color_bar_config.title),
                    );
                    if title_response.changed() {
                        color_bar_config.has_changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Unit:");
                    let unit_response = ui.add_sized(
                        [70.0, 20.0],
                        egui::TextEdit::singleline(&mut color_bar_config.unit),
                    );
                    if unit_response.changed() {
                        color_bar_config.has_changed = true;
                    }
                });

                ui.separator();

                // 隐藏颜色条按钮
                if ui
                    .add_sized([100.0, 25.0], egui::Button::new("Hide Color Bar"))
                    .clicked()
                {
                    color_bar_config.visible = false;
                }
            });
        });
}

/// 使用简单布局渲染颜色渐变条
///
/// 避免复杂的绝对定位，使用自然的UI布局
fn render_color_gradient_simple(ui: &mut egui::Ui, color_map: &ColorMap, config: &ColorBarConfig) {
    // 固定尺寸
    let bar_width = 30.0;
    let bar_height = 250.0;

    // 确保最小值小于最大值
    let min_val = config.min_value.min(config.max_value);
    let max_val = config.min_value.max(config.max_value);
    let value_range = max_val - min_val;

    // 标题
    if !config.title.is_empty() {
        ui.label(&config.title);
        ui.add_space(5.0);
    }

    // 颜色条和标签水平布局
    ui.horizontal(|ui| {
        // 颜色条
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::new(bar_width, bar_height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // 绘制颜色渐变
            let segments = 50;
            let segment_height = bar_height / segments as f32;

            for i in 0..segments {
                let t = 1.0 - (i as f32 / (segments - 1) as f32);
                let color_rgba = color_map.get_interpolated_color(t);

                let color = egui::Color32::from_rgba_premultiplied(
                    (color_rgba[0] * 255.0) as u8,
                    (color_rgba[1] * 255.0) as u8,
                    (color_rgba[2] * 255.0) as u8,
                    (color_rgba[3] * 255.0) as u8,
                );

                let segment_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(rect.min.x, rect.min.y + i as f32 * segment_height),
                    egui::Vec2::new(bar_width, segment_height + 1.0),
                );

                painter.rect_filled(segment_rect, 0.0, color);
            }

            // 绘制边框
            painter.rect_stroke(rect, 1.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
        }

        ui.add_space(8.0);

        // 数值标签
        ui.vertical(|ui| {
            let format_value = |val: f32, unit: &str| {
                if val.abs() < 1000.0 {
                    format!("{:.2}{}", val, unit)
                } else {
                    format!("{:.1e}{}", val, unit)
                }
            };

            // 最大值（顶部）
            ui.label(format_value(max_val, &config.unit));

            // 固定间距
            ui.add_space(95.0);

            // 中间值
            let mid_val = min_val + value_range * 0.5;
            ui.label(format_value(mid_val, &config.unit));

            // 固定间距
            ui.add_space(95.0);

            // 最小值（底部）
            ui.label(format_value(min_val, &config.unit));
        });
    });
}

/// 监听颜色条配置变化并应用到现有网格
///
/// 当用户在颜色条UI中改变颜色映射表或数值范围时，
/// 这个系统会重新计算网格的顶点颜色并更新渲染
pub fn apply_color_map_changes(
    mut color_bar_config: ResMut<ColorBarConfig>,
    current_model: Res<crate::ui::CurrentModelData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_entities: Query<&Mesh3d, With<crate::ui::UserModelMesh>>,
) {
    // 只有在配置发生变化时才处理
    if !color_bar_config.has_changed {
        return;
    }

    // 重置变化标记
    color_bar_config.has_changed = false;

    // 检查是否有当前模型数据
    let Some(ref geometry) = current_model.geometry else {
        println!("No geometry data available for color map update");
        return;
    };

    println!("Applying color map changes to existing mesh...");

    // 获取用户模型网格实体并更新颜色（应该只有一个）
    if let Ok(mesh3d) = mesh_entities.get_single() {
        if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
            // 重新应用颜色映射
            let result = apply_custom_color_mapping(geometry, mesh, &color_bar_config);

            match result {
                Ok(()) => {
                    println!(
                        "Successfully updated user model colors with new color map: {}",
                        color_bar_config.color_map_name
                    );
                }
                Err(e) => {
                    println!("Failed to apply color mapping: {:?}", e);
                }
            }
        } else {
            println!("Could not access user model mesh for color update");
        }
    } else {
        println!("No user model entity found for color update");
    }
}

/// 应用自定义颜色映射到网格
///
/// 使用颜色条配置中指定的颜色映射表和数值范围重新计算网格颜色
pub fn apply_custom_color_mapping(
    geometry: &mesh::GeometryData,
    mesh: &mut Mesh,
    color_bar_config: &ColorBarConfig,
) -> Result<(), mesh::VtkError> {
    use crate::mesh::vtk::{AttributeLocation, AttributeType};

    let Some(attributes) = &geometry.attributes else {
        println!("No attributes available for color mapping");
        return Ok(());
    };

    // 获取新的颜色映射表
    let color_map = get_color_map(&color_bar_config.color_map_name);

    // 首先尝试处理点标量属性
    for ((name, location), attr) in attributes.iter() {
        if let AttributeType::Scalar { num_comp, data, .. } = attr {
            if location == &AttributeLocation::Point && *num_comp == 1 {
                println!(
                    "Applying custom color mapping to point scalar attribute: {}",
                    name
                );

                // 获取网格中实际的顶点数量，而不是几何数据中的顶点数量
                let mesh_vertex_count =
                    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    {
                        positions.len()
                    } else {
                        geometry.vertices.len()
                    };

                let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; mesh_vertex_count];

                // 使用自定义数值范围
                let min_val = color_bar_config.min_value;
                let max_val = color_bar_config.max_value;
                let range = max_val - min_val;

                if range <= 0.0 {
                    println!("Constant scalar field detected (range={}), using middle color from color map", range);
                    // 对于常数场，使用颜色映射表的中间颜色
                    let middle_color = color_map.get_interpolated_color(0.5);
                    vertex_colors.fill(middle_color);

                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        bevy::render::mesh::VertexAttributeValues::from(vertex_colors),
                    );
                    println!(
                        "Applied middle color [{:.3}, {:.3}, {:.3}, {:.3}] to constant point field",
                        middle_color[0], middle_color[1], middle_color[2], middle_color[3]
                    );
                    return Ok(());
                }

                // 为每个顶点计算颜色
                for (i, &val) in data.iter().enumerate() {
                    if i >= vertex_colors.len() {
                        break;
                    }

                    // 使用自定义范围进行归一化
                    let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
                    let color = color_map.get_interpolated_color(normalized);
                    vertex_colors[i] = color;
                }

                println!(
                    "Applied colors to {} vertices (geometry has {} vertices, {} scalar values)",
                    vertex_colors.len(),
                    geometry.vertices.len(),
                    data.len()
                );

                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    bevy::render::mesh::VertexAttributeValues::from(vertex_colors),
                );
                println!(
                    "Updated point scalar colors with color map: {}",
                    color_bar_config.color_map_name
                );
                return Ok(());
            }
        }
    }

    // 如果没有点标量，尝试处理单元格标量属性
    for ((name, location), attr) in attributes.iter() {
        if let AttributeType::Scalar { num_comp, data, .. } = attr {
            if location == &AttributeLocation::Cell && *num_comp == 1 {
                println!(
                    "Applying custom color mapping to cell scalar attribute: {}",
                    name
                );

                // 获取网格中实际的顶点数量，而不是几何数据中的顶点数量
                let mesh_vertex_count =
                    if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(positions)) =
                        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                    {
                        positions.len()
                    } else {
                        geometry.vertices.len()
                    };

                let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; mesh_vertex_count];

                // 使用自定义数值范围
                let min_val = color_bar_config.min_value;
                let max_val = color_bar_config.max_value;
                let range = max_val - min_val;

                if range <= 0.0 {
                    println!("Constant scalar field detected (range={}), using middle color from color map", range);
                    // 对于常数场，使用颜色映射表的中间颜色
                    let middle_color = color_map.get_interpolated_color(0.5);
                    vertex_colors.fill(middle_color);

                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        bevy::render::mesh::VertexAttributeValues::from(vertex_colors),
                    );
                    println!(
                        "Applied middle color [{:.3}, {:.3}, {:.3}, {:.3}] to constant cell field",
                        middle_color[0], middle_color[1], middle_color[2], middle_color[3]
                    );
                    return Ok(());
                }

                // 使用三角形到单元格的映射
                if let Some(mapping) = &geometry.triangle_to_cell_mapping {
                    for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                        if cell_idx >= data.len() {
                            continue;
                        }

                        // 计算颜色
                        let val = data[cell_idx];
                        let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
                        let color = color_map.get_interpolated_color(normalized);

                        // 获取三角形的顶点索引并设置颜色
                        let triangle_base = triangle_idx * 3;
                        if triangle_base + 2 < geometry.indices.len() {
                            let vertex_indices = [
                                geometry.indices[triangle_base] as usize,
                                geometry.indices[triangle_base + 1] as usize,
                                geometry.indices[triangle_base + 2] as usize,
                            ];

                            for &idx in &vertex_indices {
                                if idx < vertex_colors.len() {
                                    vertex_colors[idx] = color;
                                }
                            }
                        }
                    }
                }

                println!(
                    "Applied cell colors to {} vertices (geometry has {} vertices, {} triangles, {} cells)",
                    vertex_colors.len(),
                    geometry.vertices.len(),
                    geometry.indices.len() / 3,
                    data.len()
                );

                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    bevy::render::mesh::VertexAttributeValues::from(vertex_colors),
                );
                println!(
                    "Updated cell scalar colors with color map: {}",
                    color_bar_config.color_map_name
                );
                return Ok(());
            }
        }
    }

    println!("No suitable scalar attributes found for color mapping");
    Ok(())
}

/// 从几何数据中自动更新颜色条的数值范围
///
/// 分析几何数据中的标量属性，找到数值范围并自动设置到颜色条配置中
pub fn update_color_bar_range_from_geometry(
    geometry: &mesh::GeometryData,
    color_bar_config: &mut ColorBarConfig,
) {
    use crate::mesh::vtk::{AttributeLocation, AttributeType};

    let Some(attributes) = &geometry.attributes else {
        println!("No attributes available for automatic color bar range update");
        return;
    };

    // 尝试找到第一个标量属性来设置数值范围
    for ((name, location), attr) in attributes.iter() {
        if let AttributeType::Scalar { num_comp, data, .. } = attr {
            if *num_comp == 1 && !data.is_empty() {
                // 计算数值范围
                let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

                if min_val != f32::INFINITY && max_val != f32::NEG_INFINITY {
                    // 处理常数场的情况
                    if (max_val - min_val).abs() < 1e-10 {
                        // 对于常数场，设置一个对称的范围
                        let center = min_val;
                        let spread = if center.abs() > 1e-6 {
                            center.abs() * 0.1
                        } else {
                            1.0
                        };
                        color_bar_config.min_value = center - spread;
                        color_bar_config.max_value = center + spread;

                        println!(
                            "Constant field detected (value={:.3}), set range: min={:.3}, max={:.3}",
                            center, color_bar_config.min_value, color_bar_config.max_value
                        );
                    } else {
                        // 正常的变化数据
                        color_bar_config.min_value = min_val;
                        color_bar_config.max_value = max_val;

                        println!(
                            "Auto-updated color bar range from {}: min={:.3}, max={:.3}",
                            name, min_val, max_val
                        );
                    }

                    // 设置合适的标题
                    match location {
                        AttributeLocation::Point => {
                            color_bar_config.title = format!("{} (Point)", name);
                        }
                        AttributeLocation::Cell => {
                            color_bar_config.title = format!("{} (Cell)", name);
                        }
                    }

                    return;
                }
            }
        }
    }

    println!("No suitable scalar attributes found for automatic color bar range update");
}
