//! # 模型变换控制模块
//!
//! 本模块提供简单的模型变换功能：
//! - Alt + 鼠标左键拖拽：围绕模型中心旋转
//! - Alt + 鼠标中键拖拽：平移模型位置
//! - Alt + R：重置模型变换

use crate::ui::{ModelLoadedEvent, UserModelMesh};
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseButton};
use bevy::prelude::*;

// 鼠标操作敏感度
const MOUSE_ROTATION_SENSITIVITY: f32 = 0.008;
const MOUSE_TRANSLATION_SENSITIVITY: f32 = 0.1;

/// 保存模型的真正几何中心
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

/// 监听模型加载事件，更新模型几何中心
fn update_model_center(
    mut model_loaded_events: EventReader<ModelLoadedEvent>,
    mut model_center: ResMut<ModelCenter>,
) {
    for event in model_loaded_events.read() {
        if let (Some(min), Some(max)) = (event.bounds_min, event.bounds_max) {
            // 计算真正的几何中心
            model_center.center = (min + max) / 2.0;
            println!(
                "Model geometric center updated to: {:?}",
                model_center.center
            );
        }
    }
}

/// 模型变换输入处理系统
///
/// 控制方式：
/// - Alt + 左键拖拽：围绕模型几何中心旋转
/// - Alt + 中键拖拽：平移模型
/// - Alt + R：重置所有变换
fn model_transform_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    model_center: Res<ModelCenter>,
    mut model_query: Query<&mut Transform, With<UserModelMesh>>,
) {
    let alt_pressed =
        keyboard_input.pressed(KeyCode::AltLeft) || keyboard_input.pressed(KeyCode::AltRight);

    // Alt + R 重置变换
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

/// 应用围绕几何中心点的旋转
fn apply_center_rotation(transform: &mut Transform, mouse_delta: Vec2, center: Vec3) {
    // 计算旋转角度 (改为正向，让鼠标移动方向与模型旋转方向一致)
    let yaw = mouse_delta.x * MOUSE_ROTATION_SENSITIVITY;
    let pitch = mouse_delta.y * MOUSE_ROTATION_SENSITIVITY;

    // 创建旋转四元数
    let yaw_rotation = Quat::from_rotation_y(yaw);
    let pitch_rotation = Quat::from_rotation_x(pitch);
    let rotation = yaw_rotation * pitch_rotation;

    // 围绕几何中心旋转：平移到原点 -> 旋转 -> 平移回去
    let relative_pos = transform.translation - center;
    let rotated_pos = rotation * relative_pos;
    transform.translation = center + rotated_pos;

    // 应用旋转
    transform.rotation = rotation * transform.rotation;
}

/// 应用平移
fn apply_translation(transform: &mut Transform, mouse_delta: Vec2) {
    let translation = Vec3::new(
        mouse_delta.x * MOUSE_TRANSLATION_SENSITIVITY,
        -mouse_delta.y * MOUSE_TRANSLATION_SENSITIVITY,
        0.0,
    );
    transform.translation += translation;
}
