use crate::ui::ModelLoadedEvent;
use bevy::input::{
    mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    ButtonInput,
};
use bevy::prelude::*;

const MOVEMENT_SPEED: f32 = 5.0;
const ZOOM_SPEED: f32 = 20.0;
const CAMERA_DISTANCE_FACTOR: f32 = 8.0; // 增大基础距离系数

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
            .add_systems(Update, camera_controller)
            .add_systems(Update, focus_camera_on_model);
    }
}

fn spawn_camera(mut commands: Commands) {
    // Starting position slightly elevated and back from the origin
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

fn focus_camera_on_model(
    mut model_loaded_events: EventReader<ModelLoadedEvent>,
    mut camera_query: Query<
        (&mut Transform, &mut CameraRotationController),
        With<WorldModelCamera>,
    >,
) {
    for event in model_loaded_events.read() {
        if let Ok((mut camera_transform, mut rotation_controller)) = camera_query.get_single_mut() {
            // 获取模型位置
            let model_position = event.position;

            // 计算模型的大小和中心点
            let (model_size, model_center) =
                if let (Some(min), Some(max)) = (event.bounds_min, event.bounds_max) {
                    // 如果有包围盒信息，使用包围盒计算
                    let size = (max - min).length();
                    let diagonal = max - min;
                    let max_dimension = diagonal.max_element();
                    // 使用最大维度作为模型大小，确保模型完全在视野内
                    (max_dimension, (min + max) / 2.0)
                } else {
                    // 否则使用缩放和位置估计
                    let size = event.scale.max_element().max(1.0) * 2.0;
                    (size, model_position)
                };

            // 计算合适的相机距离（基于模型大小）
            let camera_distance = model_size * CAMERA_DISTANCE_FACTOR;

            // 使用更高的视角
            let offset = Vec3::new(0.8, 1.2, 0.8).normalize() * camera_distance;
            let camera_position = model_center + offset;

            // 更新相机变换
            camera_transform.translation = camera_position;

            // 让相机看向模型中心
            camera_transform.look_at(model_center, Vec3::Y);

            // 从相机的旋转中提取欧拉角，更新控制器的角度
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

        if keyboard_input.pressed(KeyCode::KeyQ) {
            movement += transform.up() * MOVEMENT_SPEED;
        }

        if keyboard_input.pressed(KeyCode::KeyE) {
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
