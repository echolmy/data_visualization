//! # Model Transform Control Module
//!
//! This module provides simple model transformation functionality:
//! - Alt + Left mouse drag: Rotate around model center
//! - Alt + Middle mouse drag: Translate model position
//! - Alt + R: Reset model transform

use crate::ui::{ModelLoadedEvent, UserModelMesh};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseButton};
use bevy::prelude::*;

// Mouse operation sensitivity
const MOUSE_ROTATION_SENSITIVITY: f32 = 0.005;
const MOUSE_TRANSLATION_SENSITIVITY: f32 = 0.1;

/// Store model geometric center
#[derive(Resource, Default)]
pub struct ModelCenter {
    pub center: Vec3,
}

pub struct ModelTransformPlugin;

impl Plugin for ModelTransformPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModelCenter>()
            .add_systems(Update, (model_transform_input, update_model_center));
    }
}

/// Listen for model loading events and update model geometric center
fn update_model_center(
    mut model_loaded_events: EventReader<ModelLoadedEvent>,
    mut model_center: ResMut<ModelCenter>,
) {
    for event in model_loaded_events.read() {
        if let (Some(min), Some(max)) = (event.bounds_min, event.bounds_max) {
            // Calculate geometric center
            model_center.center = (min + max) / 2.0;
            println!(
                "Model geometric center updated to: {:?}",
                model_center.center
            );
        }
    }
}

/// Model transform input handling system
///
/// Control scheme:
/// - Alt + Left drag: Rotate around model geometric center
/// - Alt + Middle drag: Translate model
/// - Alt + R: Reset all transforms
fn model_transform_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    model_center: Res<ModelCenter>,
    mut model_query: Query<&mut Transform, With<UserModelMesh>>,
) {
    let alt_pressed =
        keyboard_input.pressed(KeyCode::AltLeft) || keyboard_input.pressed(KeyCode::AltRight);

    // Alt + R reset transform
    if alt_pressed && keyboard_input.just_pressed(KeyCode::KeyR) {
        for mut transform in model_query.iter_mut() {
            *transform = Transform::IDENTITY;
            println!("Model transform reset");
        }
        return;
    }

    if !alt_pressed || accumulated_mouse_motion.delta == Vec2::ZERO {
        return;
    }

    let should_rotate = mouse_button_input.pressed(MouseButton::Left);
    let should_translate = mouse_button_input.pressed(MouseButton::Middle);

    if !should_rotate && !should_translate {
        return;
    }

    for mut transform in model_query.iter_mut() {
        if should_rotate {
            apply_center_rotation(
                &mut transform,
                accumulated_mouse_motion.delta,
                model_center.center,
            );
        } else if should_translate {
            apply_translation(&mut transform, accumulated_mouse_motion.delta);
        }
    }
}

/// Apply rotation around geometric center point
fn apply_center_rotation(transform: &mut Transform, mouse_delta: Vec2, center: Vec3) {
    // Calculate rotation angles
    let yaw = mouse_delta.x * MOUSE_ROTATION_SENSITIVITY;
    let pitch = mouse_delta.y * MOUSE_ROTATION_SENSITIVITY;

    // Create rotation quaternions
    let yaw_rotation = Quat::from_rotation_y(yaw);
    let pitch_rotation = Quat::from_rotation_x(pitch);
    let rotation = yaw_rotation * pitch_rotation;

    // Rotate around geometric center
    let relative_pos = transform.translation - center;
    let rotated_pos = rotation * relative_pos;
    transform.translation = center + rotated_pos;

    // Apply rotation
    transform.rotation = rotation * transform.rotation;
}

/// Apply translation
fn apply_translation(transform: &mut Transform, mouse_delta: Vec2) {
    let translation = Vec3::new(
        mouse_delta.x * MOUSE_TRANSLATION_SENSITIVITY,
        -mouse_delta.y * MOUSE_TRANSLATION_SENSITIVITY,
        0.0,
    );
    transform.translation += translation;
}
