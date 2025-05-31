//! Wireframe Rendering Module
//!
//! Implements wireframe rendering functionality using the Bevy engine, providing global wireframe mode
//! toggling and individual entity wireframe control. Supports keyboard shortcuts (Z key) and UI events
//! to toggle wireframe rendering mode.

use crate::ui::events::ToggleWireframeEvent;
use crate::Mesh3d;
use bevy::{
    color::palettes::css::*,
    pbr::wireframe::{NoWireframe, WireframeConfig, WireframePlugin},
    prelude::*,
};

/// Component for tracking mesh entities that have been processed for wireframe rendering
///
/// This component is used to track which mesh entities have already been processed for wireframe
/// rendering settings, avoiding duplicate processing of the same entity.
#[derive(Component)]
pub struct ProcessedForWireframe;

/// Wireframe Rendering Plugin
///
/// A Bevy plugin that provides wireframe rendering functionality, including:
/// - Global wireframe mode configuration
/// - Default wireframe color settings  
/// - Wireframe toggle system
pub struct WireframeRenderPlugin;

impl Plugin for WireframeRenderPlugin {
    fn build(&self, app: &mut App) {
        info!("Initialize WireframeRenderPlugin");
        app.add_plugins(WireframePlugin)
            .insert_resource(WireframeConfig {
                // default not enable global wireframe mode
                global: false,
                // control the default color of all wireframe
                default_color: WHITE.into(),
            })
            .add_systems(Update, toggle_wireframe);
    }
}

/// System for toggling wireframe rendering settings
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
fn toggle_wireframe(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut wireframe_toggle_events: EventReader<ToggleWireframeEvent>,
    mut config: ResMut<WireframeConfig>,
    query: Query<(Entity, Option<&NoWireframe>, Option<&ProcessedForWireframe>), With<Mesh3d>>,
) {
    // check how many entities can render wireframe
    let mesh_count = query.iter().count();

    // if it's the first time running, output some information
    static FIRST_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    if FIRST_RUN.swap(false, std::sync::atomic::Ordering::Relaxed) {
        info!(
            "Wireframe toggle system started, detected {} entities with Mesh3d",
            mesh_count
        );
    }

    // toggle global wireframe mode by pressing Z key or UI button
    let should_toggle = keyboard_input.just_pressed(KeyCode::KeyZ)
        || wireframe_toggle_events.read().next().is_some();

    if should_toggle {
        config.global = !config.global;
        info!(
            "Toggle global wireframe mode: {}",
            if config.global { "on" } else { "off" }
        );
    }
}
