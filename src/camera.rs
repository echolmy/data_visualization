use bevy::input::{
    mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    ButtonInput,
};
use bevy::{prelude::*, window::PrimaryWindow};

const MOVEMENT_SPEED: f32 = 5.0;
const ZOOM_SPEED: f32 = 20.0;

#[derive(Debug, Component)]
struct WorldModelCamera;

#[derive(Component)]
struct CameraRotationController {
    // rotation sensitivity
    sensitivity: f32,

    // current camera angle
    // yaw: horizontal
    // pitch: vertical
    yaw: f32,
    pitch: f32,

    // confine range of rotation
    max_pitch: f32,
    min_pitch: f32,
}

impl Default for CameraRotationController {
    fn default() -> Self {
        Self {
            sensitivity: 0.01,
            yaw: 0.0,
            pitch: 0.0,
            max_pitch: std::f32::consts::FRAC_PI_2 * 0.9, // about angle of 80 degree
            min_pitch: -std::f32::consts::FRAC_PI_2 * 0.9,
        }
    }
}

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(Update, camera_controller);
    }
}

fn spawn_camera(mut commands: Commands, window_query: Query<&Window, With<PrimaryWindow>>) {
    let window = window_query.get_single().unwrap();
    commands.spawn((
        WorldModelCamera,
        CameraRotationController::default(),
        Camera3d::default(),
        Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, 0.0),
        Projection::from(PerspectiveProjection {
            fov: 90.0_f32.to_radians(),
            ..default()
        }),
    ));
}

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

        // translation
        // keyboard input
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

        if keyboard_input.pressed(KeyCode::Space) {
            movement += transform.up() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyC) {
            movement += transform.down() * MOVEMENT_SPEED;
        }

        // mouse scroll
        if accumulated_mouse_scroll.delta != Vec2::ZERO {
            let zoom_delta = accumulated_mouse_scroll.delta.y * ZOOM_SPEED;
            movement += transform.forward() * zoom_delta;
        }

        transform.translation += movement * time.delta_secs();

        // Rotation
        if mouse_button_input.pressed(MouseButton::Right)
            && accumulated_mouse_motion.delta != Vec2::ZERO
        {
            // update horizontal rotation
            rotation_controller.yaw -=
                accumulated_mouse_motion.delta.x * rotation_controller.sensitivity;

            // update vertical rotation, confine vertical rotation range
            rotation_controller.pitch -=
                accumulated_mouse_motion.delta.y * rotation_controller.sensitivity;
            rotation_controller.pitch = rotation_controller
                .pitch
                .clamp(rotation_controller.min_pitch, rotation_controller.max_pitch);
        }

        // apply rotation to camera transform component
        let yaw_rotation = Quat::from_rotation_y(rotation_controller.yaw);
        let pitch_rotation = Quat::from_rotation_x(rotation_controller.pitch);
        transform.rotation = yaw_rotation * pitch_rotation;
    }
}
