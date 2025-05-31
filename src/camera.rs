//! # Camera Control Module
//!
//! This module provides a complete camera control system for 3D scenes, including:
//! - Free-flight camera control
//! - Mouse rotation control
//! - Keyboard movement control
//! - Mouse wheel zoom
//! - Automatic focus on loaded models
//!
//! ## Control Scheme
//!
//! ### Keyboard Controls
//! - W/↑: Move forward
//! - A/←: Move left
//! - S/↓: Move backward  
//! - D/→: Move right
//! - Q: Move up
//! - E: Move down
//!
//! ### Mouse Controls
//! - Right-click drag: Rotate view
//! - Scroll wheel: Zoom

use crate::ui::ModelLoadedEvent;
use bevy::input::{
    mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    ButtonInput,
};
use bevy::prelude::*;

/// Camera movement speed (units per second)
const MOVEMENT_SPEED: f32 = 5.0;
/// Zoom speed multiplier
const ZOOM_SPEED: f32 = 20.0;
/// Camera distance factor for calculating appropriate viewing distance from models
const CAMERA_DISTANCE_FACTOR: f32 = 5.0; // increase base distance factor

/// Component that marks the 3D world model camera
///
/// This component is used to identify the main camera in the scene for rendering 3D models and scenes.
/// It allows querying and manipulating specific camera entities within systems.
#[derive(Debug, Component)]
struct WorldModelCamera;

/// Camera rotation controller
///
/// Manages the camera's rotation state and parameters, including:
/// - Mouse sensitivity settings
/// - Current yaw and pitch angles
/// - Pitch angle constraint range
#[derive(Component)]
struct CameraRotationController {
    /// Mouse rotation sensitivity - higher values result in faster rotation
    sensitivity: f32,

    /// Current camera yaw angle (horizontal rotation)
    /// Positive values indicate rightward rotation, negative values indicate leftward rotation
    yaw: f32,

    /// Current camera pitch angle (vertical rotation)
    /// Positive values indicate looking up, negative values indicate looking down
    pitch: f32,

    /// Maximum pitch angle (upward look limit)
    max_pitch: f32,

    /// Minimum pitch angle (downward look limit)
    min_pitch: f32,
}

impl Default for CameraRotationController {
    /// Creates a default camera rotation controller
    ///
    /// Default configuration:
    /// - Sensitivity: 0.01 (moderate mouse sensitivity)
    /// - Initial angles: 0 (facing forward)
    /// - Pitch limits: ±80 degrees (prevents gimbal lock)
    fn default() -> Self {
        Self {
            sensitivity: 0.01,
            yaw: 0.0,
            pitch: 0.0,
            max_pitch: std::f32::consts::FRAC_PI_2 * 0.9, // approximately 80 degrees
            min_pitch: -std::f32::consts::FRAC_PI_2 * 0.9,
        }
    }
}

/// Camera control plugin
///
/// Responsible for registering camera-related systems to the Bevy app, including:
/// - Camera spawning system
/// - Camera control system  
/// - Model focusing system
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    /// Builds the camera plugin by registering all related systems
    ///
    /// # System Registration Order
    /// 1. Startup: Spawn camera
    /// 2. Update: Camera control and model focusing (run in parallel)
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, camera_controller)
            .add_systems(Update, focus_camera_on_model);
    }
}

/// Spawns the main camera in the scene
///
/// Creates a 3D camera at scene startup with initial position and orientation:
/// - Position: (10, 10, 10) - slightly elevated and away from origin
/// - Looking at: Scene origin (0, 0, 0)
/// - Up direction: Positive Y-axis
///
/// # Parameters
/// * `commands` - Bevy's command buffer for spawning entities
fn spawn_camera(mut commands: Commands) {
    // Starting position: slightly elevated and back from the origin
    let camera_position = Vec3::new(10.0, 10.0, 10.0);

    // Look at the origin
    let look_target = Vec3::ZERO;

    commands.spawn((
        WorldModelCamera,
        CameraRotationController::default(),
        Camera3d::default(),
        Transform::from_translation(camera_position).looking_at(look_target, Vec3::Y),
    ));
}

/// Automatically adjusts camera focus to model when a model is loaded
///
/// Listens for `ModelLoadedEvent` events and automatically calculates appropriate
/// camera position and orientation based on the model's position, size, and bounding box
/// to ensure the model is fully visible.
///
/// # Calculation Logic
/// 1. Get the model's bounding box or estimate size using scale information
/// 2. Calculate the model's center point
/// 3. Calculate appropriate camera distance based on model size
/// 4. Set camera position to model center + offset
/// 5. Make camera look at model center
/// 6. Update rotation controller's angle state
///
/// # Parameters
/// * `model_loaded_events` - Event reader for model loading events
/// * `camera_query` - Query for mutable references to camera transform and rotation controller
fn focus_camera_on_model(
    mut model_loaded_events: EventReader<ModelLoadedEvent>,
    mut camera_query: Query<
        (&mut Transform, &mut CameraRotationController),
        With<WorldModelCamera>,
    >,
) {
    for event in model_loaded_events.read() {
        if let Ok((mut camera_transform, mut rotation_controller)) = camera_query.get_single_mut() {
            // Get model position
            let model_position = event.position;

            // Calculate model size and center point
            let (model_size, model_center) =
                if let (Some(min), Some(max)) = (event.bounds_min, event.bounds_max) {
                    // let size = (max - min).length();
                    let diagonal = max - min;
                    let max_dimension = diagonal.max_element();
                    // Use max dimension as model size to ensure model is fully in view
                    (max_dimension, (min + max) / 2.0)
                } else {
                    // Otherwise use scale and position estimation
                    let size = event.scale.max_element().max(1.0) * 2.0;
                    (size, model_position)
                };

            // Calculate appropriate camera distance (based on model size)
            let camera_distance = model_size * CAMERA_DISTANCE_FACTOR;

            // Use elevated viewing angle
            let offset = Vec3::new(0.8, 1.2, 0.8).normalize() * camera_distance;
            let camera_position = model_center + offset;

            // Update camera transform
            camera_transform.translation = camera_position;

            // Make camera look at model center
            camera_transform.look_at(model_center, Vec3::Y);

            // Extract euler angles from camera rotation, update controller angles
            let (pitch, yaw, _) = camera_transform.rotation.to_euler(EulerRot::XYZ);
            rotation_controller.yaw = yaw;
            rotation_controller.pitch = pitch;

            println!(
                "Camera focused on model at center: {:?}, size: {}, distance: {}",
                model_center, model_size, camera_distance
            );
        }
    }
}

/// Camera control system
///
/// Handles all camera input and movement logic, including:
///
/// ## Keyboard Movement Controls
/// - WASD/Arrow keys: Forward, left, backward, right movement
/// - QE: Up and down movement
///
/// ## Mouse Controls
/// - Scroll wheel: Forward/backward zoom
/// - Right-click drag: Rotate view (yaw and pitch)
///
/// ## Movement Calculation
/// All movement is based on the camera's current orientation for intuitive control:
/// - Forward/backward movement along camera's facing direction
/// - Left/right movement perpendicular to camera's facing direction
/// - Up/down movement along world Y-axis
///
/// ## Rotation Constraints
/// - Pitch angle is constrained to ±80 degrees to prevent camera flipping
/// - Yaw angle has no constraints, allowing 360-degree rotation
///
/// # Parameters
/// * `keyboard_input` - Keyboard input state
/// * `mouse_button_input` - Mouse button input state  
/// * `accumulated_mouse_motion` - Accumulated mouse movement delta
/// * `accumulated_mouse_scroll` - Accumulated mouse scroll input
/// * `controller_query` - Query for camera transform and rotation controller
/// * `time` - Time resource for frame-rate independent movement
fn camera_controller(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mut controller_query: Query<(&mut Transform, &mut CameraRotationController), With<Camera3d>>,
    time: Res<Time>,
) {
    if let Ok((mut transform, mut rotation_controller)) = controller_query.get_single_mut() {
        let mut movement = Vec3::ZERO;

        // Translation controls
        // Keyboard input
        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            movement += transform.forward() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            movement += transform.left() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            movement += transform.back() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            movement += transform.right() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyQ) {
            movement += transform.up() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyE) {
            movement += transform.down() * MOVEMENT_SPEED;
        }

        // Mouse scroll wheel zoom
        if accumulated_mouse_scroll.delta != Vec2::ZERO {
            let zoom_delta = accumulated_mouse_scroll.delta.y * ZOOM_SPEED;
            movement += transform.forward() * zoom_delta;
        }

        // Apply movement (frame-time based)
        transform.translation += movement * time.delta_secs();

        // Rotation controls
        if mouse_button_input.pressed(MouseButton::Right)
            && accumulated_mouse_motion.delta != Vec2::ZERO
        {
            // Update horizontal rotation (yaw)
            rotation_controller.yaw -=
                accumulated_mouse_motion.delta.x * rotation_controller.sensitivity;

            // Update vertical rotation (pitch), constrain vertical rotation range
            rotation_controller.pitch -=
                accumulated_mouse_motion.delta.y * rotation_controller.sensitivity;
            rotation_controller.pitch = rotation_controller
                .pitch
                .clamp(rotation_controller.min_pitch, rotation_controller.max_pitch);
        }

        // Apply rotation to camera transform component
        let yaw_rotation = Quat::from_rotation_y(rotation_controller.yaw);
        let pitch_rotation = Quat::from_rotation_x(rotation_controller.pitch);
        transform.rotation = yaw_rotation * pitch_rotation;
    }
}
