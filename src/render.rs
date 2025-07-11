//! Render Module
//!
//! Contains all rendering-related functionality:
//! - Wireframe rendering: Global wireframe mode toggle and individual control
//! - Wave material: Dynamic wave effects implemented with GPU shaders

// Sub-module declarations
pub mod wave_material;

// Re-export wave material related items
pub use wave_material::{animate_wave_shader, create_flat_plane_mesh, WaveMaterial};

/// Wireframe Rendering Functionality
///
/// Implements wireframe rendering functionality using the Bevy engine, providing global wireframe mode
/// toggling and individual entity wireframe control. Supports keyboard shortcuts (Z key) and UI events
/// to toggle wireframe rendering mode.
use crate::ui::events::ToggleWireframeEvent;
use crate::Mesh3d;
use bevy::{
    color::palettes::css::*,
    pbr::wireframe::{NoWireframe, WireframeConfig},
    prelude::*,
};

/// Component for tracking mesh entities that have been processed for wireframe rendering
///
/// This component is used to track which mesh entities have already been processed for wireframe
/// rendering settings, avoiding duplicate processing of the same entity.
#[derive(Component)]
pub struct ProcessedForWireframe;

/// Toggle wireframe rendering system
///
/// This system handles wireframe mode toggling with the following features:
/// - Toggle global wireframe mode via Z key press
/// - Toggle wireframe mode via UI events
/// - Automatically detect and count entities that can render wireframes
/// - Output debug information on first run
///
/// # Parameters
/// - `keyboard_input`: Keyboard input resource for detecting Z key press
/// - `wireframe_toggle_events`: Wireframe toggle event reader for handling UI toggle requests
/// - `config`: Mutable wireframe configuration resource for modifying global wireframe settings
/// - `query`: Query for all entities with Mesh3d component for counting and processing
pub fn toggle_wireframe(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut wireframe_toggle_events: EventReader<ToggleWireframeEvent>,
    mut config: ResMut<WireframeConfig>,
    query: Query<(Entity, Option<&NoWireframe>, Option<&ProcessedForWireframe>), With<Mesh3d>>,
) {
    // Check how many entities can render wireframes
    let mesh_count = query.iter().count();

    // If it's the first time running, output some information
    static FIRST_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    if FIRST_RUN.swap(false, std::sync::atomic::Ordering::Relaxed) {
        info!(
            "Wireframe toggle system started, detected {} entities with Mesh3d",
            mesh_count
        );
    }

    // Toggle global wireframe mode by pressing Z key or UI button
    let should_toggle = keyboard_input.just_pressed(KeyCode::KeyZ)
        || wireframe_toggle_events.read().next().is_some();

    if should_toggle {
        config.global = !config.global;
        info!(
            "Toggle global wireframe mode: {}",
            if config.global { "enabled" } else { "disabled" }
        );
    }
}

/// Initialize wireframe rendering configuration
///
/// Sets up default wireframe rendering configuration, including:
/// - Default global wireframe mode disabled
/// - Set default wireframe color to white
///
/// # Returns
/// Returns configured WireframeConfig resource
pub fn create_wireframe_config() -> WireframeConfig {
    WireframeConfig {
        // Default global wireframe mode disabled
        global: false,
        // Control the default color of all wireframes
        default_color: WHITE.into(),
    }
}
