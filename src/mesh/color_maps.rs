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
