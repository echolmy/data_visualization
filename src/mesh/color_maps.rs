//! # Color Mapping Module
//!
//! Provides color mapping functionality commonly used in scientific visualization,
//! for mapping scalar data values to corresponding colors.
//!
//! ## Color Map Types
//!
//! - `default`: Rainbow color map
//! - `viridis`: Perceptually uniform color map recommended for scientific visualization
//! - `hot`: Heatmap color map
//! - `cool`: Cool color map
//! - `warm`: Warm color map
use crate::mesh::vtk::{AttributeLocation, AttributeType};
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
#[derive(Debug, Clone)]
pub struct ColorMap {
    #[allow(dead_code)] // For debugging
    pub name: String,
    pub colors: Vec<[f32; 4]>,
}

impl ColorMap {
    /// Get interpolated color based on scalar value
    ///
    /// Parameters:
    /// * `value` - Normalized scalar value (0.0-1.0)
    ///
    /// Returns:
    /// * Linearly interpolated RGBA color
    pub fn get_interpolated_color(&self, value: f32) -> [f32; 4] {
        let normalized = value.clamp(0.0, 1.0);

        if self.colors.is_empty() {
            return [1.0, 1.0, 1.0, 1.0]; // Default white color
        }

        if self.colors.len() == 1 {
            return self.colors[0];
        }

        // Calculate float index
        let float_index = normalized * (self.colors.len() - 1) as f32;
        let lower_index = float_index.floor() as usize;
        let upper_index = (lower_index + 1).min(self.colors.len() - 1);

        // Return directly if exactly on boundary
        if lower_index == upper_index {
            return self.colors[lower_index];
        }

        // Calculate interpolation weight
        let weight = float_index - lower_index as f32;
        let lower_color = self.colors[lower_index];
        let upper_color = self.colors[upper_index];

        // Linear interpolation
        [
            lower_color[0] * (1.0 - weight) + upper_color[0] * weight,
            lower_color[1] * (1.0 - weight) + upper_color[1] * weight,
            lower_color[2] * (1.0 - weight) + upper_color[2] * weight,
            lower_color[3] * (1.0 - weight) + upper_color[3] * weight,
        ]
    }
}

/// Get the default color map
pub fn get_default_color_map() -> ColorMap {
    ColorMap {
        name: "default".to_string(),
        colors: vec![
            [0.0, 0.0, 0.6, 1.0],
            [0.0, 0.0, 0.7, 1.0],
            [0.0, 0.0, 0.8, 1.0],
            [0.0, 0.0, 0.9, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 0.2, 1.0, 1.0],
            [0.0, 0.4, 1.0, 1.0],
            [0.0, 0.6, 1.0, 1.0],
            [0.0, 0.8, 1.0, 1.0],
            [0.0, 1.0, 1.0, 1.0],
            [0.0, 1.0, 0.8, 1.0],
            [0.0, 1.0, 0.6, 1.0],
            [0.0, 1.0, 0.4, 1.0],
            [0.0, 1.0, 0.2, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.2, 1.0, 0.0, 1.0],
            [0.4, 1.0, 0.0, 1.0],
            [0.6, 1.0, 0.0, 1.0],
            [0.8, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [1.0, 0.6, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ],
    }
}

/// Get the hot color map for heatmap visualization
pub fn get_hot_color_map() -> ColorMap {
    ColorMap {
        name: "hot".to_string(),
        colors: vec![
            [0.0, 0.0, 0.0, 1.0],
            [0.1, 0.0, 0.0, 1.0],
            [0.2, 0.0, 0.0, 1.0],
            [0.3, 0.0, 0.0, 1.0],
            [0.4, 0.0, 0.0, 1.0],
            [0.5, 0.0, 0.0, 1.0],
            [0.6, 0.0, 0.0, 1.0],
            [0.7, 0.0, 0.0, 1.0],
            [0.8, 0.0, 0.0, 1.0],
            [0.9, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.1, 0.0, 1.0],
            [1.0, 0.2, 0.0, 1.0],
            [1.0, 0.3, 0.0, 1.0],
            [1.0, 0.4, 0.0, 1.0],
            [1.0, 0.5, 0.0, 1.0],
            [1.0, 0.6, 0.0, 1.0],
            [1.0, 0.7, 0.0, 1.0],
            [1.0, 0.8, 0.0, 1.0],
            [1.0, 0.9, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ],
    }
}

/// Get the Viridis color map commonly used in scientific visualization
pub fn get_viridis_color_map() -> ColorMap {
    ColorMap {
        name: "viridis".to_string(),
        colors: vec![
            [0.267004, 0.004874, 0.329415, 1.0],
            [0.275191, 0.060826, 0.390374, 1.0],
            [0.282623, 0.140926, 0.457517, 1.0],
            [0.285109, 0.195242, 0.495702, 1.0],
            [0.253935, 0.265254, 0.529983, 1.0],
            [0.230341, 0.318626, 0.545695, 1.0],
            [0.206756, 0.371758, 0.553117, 1.0],
            [0.184586, 0.423943, 0.556295, 1.0],
            [0.163625, 0.471133, 0.558148, 1.0],
            [0.144544, 0.516775, 0.557885, 1.0],
            [0.127568, 0.566949, 0.550556, 1.0],
            [0.131109, 0.616355, 0.533488, 1.0],
            [0.134692, 0.658636, 0.517649, 1.0],
            [0.177423, 0.699873, 0.490448, 1.0],
            [0.266941, 0.748751, 0.440573, 1.0],
            [0.369214, 0.788888, 0.382914, 1.0],
            [0.477504, 0.821444, 0.318195, 1.0],
            [0.590330, 0.851556, 0.248701, 1.0],
            [0.706680, 0.877588, 0.175630, 1.0],
            [0.741388, 0.873449, 0.149561, 1.0],
            [0.865006, 0.897915, 0.145833, 1.0],
            [0.993248, 0.906157, 0.143936, 1.0],
        ],
    }
}

/// Get the cool tone color map (blue to cyan series)
pub fn get_cool_color_map() -> ColorMap {
    ColorMap {
        name: "cool".to_string(),
        colors: vec![
            [0.0, 0.0, 0.3, 1.0],
            [0.0, 0.0, 0.4, 1.0],
            [0.0, 0.0, 0.5, 1.0],
            [0.0, 0.0, 0.6, 1.0],
            [0.0, 0.0, 0.7, 1.0],
            [0.0, 0.0, 0.8, 1.0],
            [0.0, 0.0, 0.9, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [0.0, 0.1, 1.0, 1.0],
            [0.0, 0.2, 1.0, 1.0],
            [0.0, 0.3, 1.0, 1.0],
            [0.0, 0.4, 1.0, 1.0],
            [0.0, 0.5, 1.0, 1.0],
            [0.0, 0.6, 1.0, 1.0],
            [0.0, 0.7, 1.0, 1.0],
            [0.0, 0.8, 1.0, 1.0],
            [0.0, 0.9, 1.0, 1.0],
            [0.0, 1.0, 1.0, 1.0],
            [0.2, 1.0, 1.0, 1.0],
            [0.4, 1.0, 1.0, 1.0],
            [0.6, 1.0, 1.0, 1.0],
            [0.8, 1.0, 1.0, 1.0],
        ],
    }
}

/// Get the warm tone color map (red to yellow series)
pub fn get_warm_color_map() -> ColorMap {
    ColorMap {
        name: "warm".to_string(),
        colors: vec![
            [0.4, 0.0, 0.0, 1.0],
            [0.5, 0.0, 0.0, 1.0],
            [0.6, 0.0, 0.0, 1.0],
            [0.7, 0.0, 0.0, 1.0],
            [0.8, 0.0, 0.0, 1.0],
            [0.9, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.1, 0.0, 1.0],
            [1.0, 0.2, 0.0, 1.0],
            [1.0, 0.3, 0.0, 1.0],
            [1.0, 0.4, 0.0, 1.0],
            [1.0, 0.5, 0.0, 1.0],
            [1.0, 0.6, 0.0, 1.0],
            [1.0, 0.7, 0.0, 1.0],
            [1.0, 0.8, 0.0, 1.0],
            [1.0, 0.9, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.2, 1.0],
            [1.0, 1.0, 0.4, 1.0],
            [1.0, 1.0, 0.6, 1.0],
            [1.0, 1.0, 0.8, 1.0],
            [1.0, 1.0, 1.0, 1.0],
        ],
    }
}

/// Get color map by name
pub fn get_color_map(name: &str) -> ColorMap {
    match name {
        "viridis" => get_viridis_color_map(),
        "hot" => get_hot_color_map(),
        "cool" => get_cool_color_map(),
        "warm" => get_warm_color_map(),
        _ => get_default_color_map(),
    }
}

// ============================================================================
// Color Mapping Functions
// ============================================================================

/// Configuration for color mapping
#[derive(Debug, Clone)]
pub struct ColorMappingConfig {
    pub color_map_name: String,
    pub min_value: f32,
    pub max_value: f32,
    pub use_custom_range: bool,
}

impl Default for ColorMappingConfig {
    fn default() -> Self {
        Self {
            color_map_name: "viridis".to_string(),
            min_value: 0.0,
            max_value: 1.0,
            use_custom_range: false,
        }
    }
}

/// Color mapper
pub struct ColorMapper;

impl ColorMapper {
    /// Apply point color scalars to a mesh
    pub fn apply_point_color_scalars(
        geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
    ) -> Result<(), crate::mesh::VtkError> {
        if let Some(attributes) = &geometry.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    AttributeLocation::Point => {
                        if let AttributeType::ColorScalar { nvalues, data } = attr {
                            Some((nvalues, data))
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            if let Some((nvalues, data)) = color_scalar {
                let colors = Self::process_point_color_scalars(geometry, *nvalues, data)?;
                if !colors.is_empty() {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(colors),
                    );
                    println!("Point color scalars inserted into mesh.");
                    return Ok(());
                }
            }
        }

        println!("No point color attribute found.");
        Ok(())
    }

    /// Apply cell color scalars to a mesh
    pub fn apply_cell_color_scalars(
        geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
    ) -> Result<(), crate::mesh::VtkError> {
        if let Some(attributes) = &geometry.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    AttributeLocation::Cell => {
                        if let AttributeType::ColorScalar { nvalues, data } = attr {
                            Some((nvalues, data))
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            if let Some((nvalues, data)) = color_scalar {
                let vertex_colors = Self::process_cell_color_scalars(geometry, *nvalues, data);
                if !vertex_colors.is_empty() {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(vertex_colors),
                    );
                    println!("Cell color scalars inserted into mesh.");
                }
            }
        }
        Ok(())
    }

    /// Apply scalar attributes with color mapping
    pub fn apply_scalar_attributes_with_color_map(
        geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
        config: &ColorMappingConfig,
    ) -> Result<(), crate::mesh::VtkError> {
        if let Some(attributes) = &geometry.attributes {
            // Try point scalars first
            if Self::apply_point_scalars_with_color_map(geometry, mesh, attributes, config)? {
                return Ok(());
            }

            // Then try cell scalars
            if Self::apply_cell_scalars_with_color_map(geometry, mesh, attributes, config)? {
                return Ok(());
            }

            // Finally try color scalars
            if Self::apply_color_scalars(geometry, mesh, attributes)? {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Apply scalar values to mesh vertex colors (for animation)
    pub fn apply_scalars_to_mesh(mesh: &mut Mesh, scalars: &[f32], config: &ColorMappingConfig) {
        let vertex_count = mesh.count_vertices();

        if scalars.len() != vertex_count {
            println!(
                "Warning: Scalar data count ({}) does not match vertex count ({})",
                scalars.len(),
                vertex_count
            );
            return;
        }

        let (min_val, max_val) = if config.use_custom_range {
            (config.min_value, config.max_value)
        } else {
            scalars
                .iter()
                .fold((f32::MAX, f32::MIN), |(min, max), &val| {
                    (min.min(val), max.max(val))
                })
        };

        let color_map = get_color_map(&config.color_map_name);
        let colors = Self::map_scalars_to_colors(scalars, min_val, max_val, &color_map);

        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    // Private helper methods
    fn process_point_color_scalars(
        geometry: &crate::mesh::GeometryData,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Result<Vec<[f32; 4]>, crate::mesh::VtkError> {
        if data.len() != geometry.vertices.len() {
            println!(
                "Warning: color data number({}) does not match vertex number({})",
                data.len(),
                geometry.vertices.len()
            );
        }

        let mut colors = Vec::with_capacity(geometry.vertices.len());

        for (idx, color_data) in data.iter().enumerate() {
            if idx >= geometry.vertices.len() {
                break;
            }

            let color = match nvalues {
                3 => [color_data[0], color_data[1], color_data[2], 1.0],
                4 => [color_data[0], color_data[1], color_data[2], color_data[3]],
                _ => [1.0, 1.0, 1.0, 1.0],
            };

            colors.push(color);
        }

        if colors.len() < geometry.vertices.len() {
            colors.resize(geometry.vertices.len(), [1.0, 1.0, 1.0, 1.0]);
        }

        Ok(colors)
    }

    fn process_cell_color_scalars(
        geometry: &crate::mesh::GeometryData,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Vec<[f32; 4]> {
        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; geometry.vertices.len()];

        if let Some(mapping) = &geometry.triangle_to_cell_mapping {
            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                if cell_idx >= data.len() {
                    continue;
                }

                let triangle_base = triangle_idx * 3;
                if triangle_base + 2 >= geometry.indices.len() {
                    continue;
                }

                let vertex_indices = [
                    geometry.indices[triangle_base] as usize,
                    geometry.indices[triangle_base + 1] as usize,
                    geometry.indices[triangle_base + 2] as usize,
                ];

                let colors = &data[cell_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0],
                };

                for &idx in &vertex_indices {
                    if idx < vertex_colors.len() {
                        vertex_colors[idx] = color;
                    }
                }
            }
        } else {
            let num_triangles = geometry.indices.len() / 3;
            for triangle_idx in 0..num_triangles {
                if triangle_idx >= data.len() {
                    break;
                }

                let vertex_indices = [
                    geometry.indices[triangle_idx * 3] as usize,
                    geometry.indices[triangle_idx * 3 + 1] as usize,
                    geometry.indices[triangle_idx * 3 + 2] as usize,
                ];

                let colors = &data[triangle_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0],
                };

                for &idx in &vertex_indices {
                    if idx < vertex_colors.len() {
                        vertex_colors[idx] = color;
                    }
                }
            }
        }

        vertex_colors
    }

    fn apply_point_scalars_with_color_map(
        _geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
        attributes: &bevy::utils::HashMap<
            (String, crate::mesh::vtk::AttributeLocation),
            crate::mesh::vtk::AttributeType,
        >,
        config: &ColorMappingConfig,
    ) -> Result<bool, crate::mesh::VtkError> {
        for ((name, location), attr) in attributes.iter() {
            if let AttributeType::Scalar { num_comp, data, .. } = attr {
                if location == &AttributeLocation::Point && *num_comp == 1 {
                    println!("Applying color mapping to point scalar attribute: {}", name);

                    let mesh_vertex_count = mesh.count_vertices();
                    let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; mesh_vertex_count];

                    let (min_val, max_val) = if config.use_custom_range {
                        (config.min_value, config.max_value)
                    } else {
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        (min_val, max_val)
                    };

                    let range = max_val - min_val;
                    let color_map = get_color_map(&config.color_map_name);

                    for (i, &val) in data.iter().enumerate() {
                        if i < vertex_colors.len() {
                            let color = if range < 1e-10 {
                                color_map.get_interpolated_color(0.5)
                            } else {
                                let normalized = (val - min_val) / range;
                                color_map.get_interpolated_color(normalized)
                            };
                            vertex_colors[i] = color;
                        }
                    }

                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(vertex_colors),
                    );
                    println!("Point scalar colors applied to mesh");
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn apply_cell_scalars_with_color_map(
        geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
        attributes: &bevy::utils::HashMap<
            (String, crate::mesh::vtk::AttributeLocation),
            crate::mesh::vtk::AttributeType,
        >,
        config: &ColorMappingConfig,
    ) -> Result<bool, crate::mesh::VtkError> {
        for ((name, location), attr) in attributes.iter() {
            if let AttributeType::Scalar { num_comp, data, .. } = attr {
                if location == &AttributeLocation::Cell && *num_comp == 1 {
                    println!("Applying color mapping to cell scalar attribute: {}", name);

                    let mesh_vertex_count = mesh.count_vertices();
                    let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; mesh_vertex_count];

                    let (min_val, max_val) = if config.use_custom_range {
                        (config.min_value, config.max_value)
                    } else {
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        (min_val, max_val)
                    };

                    let range = max_val - min_val;
                    let color_map = get_color_map(&config.color_map_name);

                    if let Some(mapping) = &geometry.triangle_to_cell_mapping {
                        for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                            if cell_idx >= data.len() {
                                continue;
                            }

                            let val = data[cell_idx];
                            let color = if range < 1e-10 {
                                color_map.get_interpolated_color(0.5)
                            } else {
                                let normalized = (val - min_val) / range;
                                color_map.get_interpolated_color(normalized)
                            };

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

                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(vertex_colors),
                    );
                    println!("Cell scalar colors applied to mesh");
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn apply_color_scalars(
        geometry: &crate::mesh::GeometryData,
        mesh: &mut Mesh,
        attributes: &bevy::utils::HashMap<
            (String, crate::mesh::vtk::AttributeLocation),
            crate::mesh::vtk::AttributeType,
        >,
    ) -> Result<bool, crate::mesh::VtkError> {
        for ((_, location), attr) in attributes.iter() {
            if let AttributeType::ColorScalar { nvalues, data } = attr {
                match location {
                    AttributeLocation::Point => {
                        let colors = Self::process_point_color_scalars(geometry, *nvalues, data)?;
                        if !colors.is_empty() {
                            mesh.insert_attribute(
                                Mesh::ATTRIBUTE_COLOR,
                                VertexAttributeValues::from(colors),
                            );
                            println!("Point color scalars applied to mesh");
                            return Ok(true);
                        }
                    }
                    AttributeLocation::Cell => {
                        let colors = Self::process_cell_color_scalars(geometry, *nvalues, data);
                        if !colors.is_empty() {
                            mesh.insert_attribute(
                                Mesh::ATTRIBUTE_COLOR,
                                VertexAttributeValues::from(colors),
                            );
                            println!("Cell color scalars applied to mesh");
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    fn map_scalars_to_colors(
        scalars: &[f32],
        min_val: f32,
        max_val: f32,
        color_map: &ColorMap,
    ) -> Vec<[f32; 4]> {
        let range = max_val - min_val;

        scalars
            .iter()
            .map(|&scalar| {
                let normalized = if range > 0.0 {
                    ((scalar - min_val) / range).clamp(0.0, 1.0)
                } else {
                    0.5
                };
                color_map.get_interpolated_color(normalized)
            })
            .collect()
    }
}
