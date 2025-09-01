//! Color bar UI module
//!
//! This module provides color bar functionality for displaying color mappings.
use crate::mesh;
use crate::mesh::color_maps::{get_color_map, ColorMap, ColorMapper, ColorMappingConfig};
use bevy::prelude::*;
use bevy_egui::*;

/// Color bar configuration
///
/// Manages the display state, value range, and style settings of the color bar
#[derive(Resource)]
pub struct ColorBarConfig {
    /// Whether to show the color bar
    pub visible: bool,
    /// Name of the currently used color map
    pub color_map_name: String,
    /// Minimum value of the value range
    pub min_value: f32,
    /// Maximum value of the value range
    pub max_value: f32,
    /// Color bar title
    pub title: String,
    /// Value unit
    pub unit: String,
    /// Flag indicating if configuration has changed
    pub has_changed: bool,
}

impl Default for ColorBarConfig {
    /// Create default values for color bar configuration
    fn default() -> Self {
        Self {
            visible: true,
            color_map_name: "default".to_string(),
            min_value: -1.0,
            max_value: 1.0,
            title: "value".to_string(),
            unit: "".to_string(),
            has_changed: false,
        }
    }
}

/// Color bar UI panel
///
/// Displays a color bar panel on the right side, providing the following features:
/// - Color map selection (dropdown menu)
/// - Value range control (min/max value input boxes)
/// - Color gradient bar display
/// - Label settings (title and unit)
/// - Hide color bar button
///
/// # Parameters
/// - `contexts`: egui context for rendering UI
/// - `color_bar_config`: Color bar configuration resource
pub fn render_color_bar_inline(
    contexts: &mut EguiContexts,
    mut color_bar_config: ResMut<ColorBarConfig>,
) {
    egui::SidePanel::right("color_bar_panel")
        .min_width(180.0) // Minimum width
        .max_width(180.0) // Maximum width
        .default_width(180.0) // Default width
        .resizable(false) // Disable resizing
        .show_separator_line(false) // Hide separator line
        .show(contexts.ctx_mut(), |ui| {
            // Vertical layout
            ui.vertical(|ui| {
                // ui.heading("color bar");

                ui.separator();

                // Color map selection
                ui.label("Color Map:");
                egui::ComboBox::from_id_salt("color_map")
                    .selected_text(&color_bar_config.color_map_name)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        let color_maps = ["default", "viridis", "hot", "cool", "warm"];
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

                // Value Range
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

                // Color map selection and rendering
                let color_map = get_color_map(&color_bar_config.color_map_name);
                render_color_gradient_simple(ui, &color_map, &color_bar_config);

                ui.separator();

                // Label Settings
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

                // Hide color bar button
                if ui
                    .add_sized([100.0, 25.0], egui::Button::new("Hide Color Bar"))
                    .clicked()
                {
                    color_bar_config.visible = false;
                }
            });
        });
}

/// Render color gradient bar and value labels
///
/// # Parameters
/// - `ui`: egui UI context
/// - `color_map`: Currently used color map
/// - `config`: Color bar configuration, including value range and style settings
fn render_color_gradient_simple(ui: &mut egui::Ui, color_map: &ColorMap, config: &ColorBarConfig) {
    // Fixed dimensions
    let bar_width = 30.0;
    let bar_height = 250.0;

    // Ensure minimum value is less than maximum value
    let min_val = config.min_value.min(config.max_value);
    let max_val = config.min_value.max(config.max_value);
    let value_range = max_val - min_val;

    // Title
    if !config.title.is_empty() {
        ui.label(&config.title);
        ui.add_space(5.0);
    }

    // Horizontal layout
    ui.horizontal(|ui| {
        // Color bar
        let (rect, _) =
            ui.allocate_exact_size(egui::Vec2::new(bar_width, bar_height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Draw color gradient
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

            // Draw border
            painter.rect_stroke(rect, 1.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
        }

        ui.add_space(8.0);

        // Value labels
        ui.vertical(|ui| {
            let format_value = |val: f32, unit: &str| {
                if val.abs() < 1000.0 {
                    format!("{:.2}{}", val, unit)
                } else {
                    format!("{:.1e}{}", val, unit)
                }
            };

            // Maximum value
            ui.label(format_value(max_val, &config.unit));

            // Fixed spacing
            ui.add_space(95.0);

            // Middle value
            let mid_val = min_val + value_range * 0.5;
            ui.label(format_value(mid_val, &config.unit));

            // Fixed spacing
            ui.add_space(95.0);

            // Minimum value
            ui.label(format_value(min_val, &config.unit));
        });
    });
}

/// Monitor color bar configuration changes and apply to existing meshes
///
/// Real-time monitoring of color bar configuration changes and updating mesh colors
/// - Color map type
/// - Value range (min/max values)
/// - Other configurations affecting color display
///
/// # Parameters
/// - `color_bar_config`: Color bar configuration resource
/// - `current_model`: Current model data resource
/// - `meshes`: Mesh resource collection
/// - `mesh_entities`: User model mesh entity query
pub fn apply_color_map_changes(
    mut color_bar_config: ResMut<ColorBarConfig>,
    current_model: Res<crate::ui::CurrentModelData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_entities: Query<&Mesh3d, With<crate::ui::UserModelMesh>>,
) {
    // Only update when has_changed is true
    if !color_bar_config.has_changed {
        return;
    }

    // Reset change flag
    color_bar_config.has_changed = false;

    // Check if current model data exists
    let Some(ref geometry) = current_model.geometry else {
        println!("No geometry data available for color map update");
        return;
    };

    println!("Applying color map changes to existing mesh...");

    // Get user model mesh entity and update colors
    if let Ok(mesh3d) = mesh_entities.get_single() {
        if let Some(mesh) = meshes.get_mut(&mesh3d.0) {
            // Re-apply color mapping
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

/// Apply custom color mapping to mesh
///
/// Applies the specified color map to the mesh based on color bar configuration.
///
/// # Parameters
/// - `geometry`: Geometry data containing vertices, indices, and scalar attributes
/// - `mesh`: Mesh to update colors
/// - `color_bar_config`: Color bar configuration including color map and value range
///
/// # Returns
/// - `Ok(())`: Successfully applied color mapping
/// - `Err(mesh::VtkError)`: Error occurred during processing
pub fn apply_custom_color_mapping(
    geometry: &mesh::GeometryData,
    mesh: &mut Mesh,
    color_bar_config: &ColorBarConfig,
) -> Result<(), mesh::VtkError> {
    // Convert ColorBarConfig to ColorMappingConfig
    let config = ColorMappingConfig {
        color_map_name: color_bar_config.color_map_name.clone(),
        min_value: color_bar_config.min_value,
        max_value: color_bar_config.max_value,
        use_custom_range: true, // Always use custom range from UI
    };

    ColorMapper::apply_scalar_attributes_with_color_map(geometry, mesh, &config)
}
