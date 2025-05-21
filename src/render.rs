// 使用Bevy引擎实现的线框渲染功能
// 按下快捷键可以切换线框渲染效果

use bevy::{
    color::palettes::css::*,
    pbr::wireframe::{NoWireframe, Wireframe, WireframeColor, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
};

use crate::Mesh3d;

// 用于跟踪是否已经处理过mesh实体
#[derive(Component)]
pub struct ProcessedForWireframe;

pub struct WireframeRenderPlugin;

impl Plugin for WireframeRenderPlugin {
    fn build(&self, app: &mut App) {
        info!("初始化WireframeRenderPlugin");
        app.add_plugins(WireframePlugin)
            .insert_resource(WireframeConfig {
                // 默认不启用全局线框模式
                global: false,
                // 控制所有线框的默认颜色
                default_color: WHITE.into(),
            })
            .add_systems(Update, toggle_wireframe);

        // 移除on_model_loaded系统，不再需要它
    }
}

/// 这个系统允许切换线框渲染设置
fn toggle_wireframe(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<WireframeConfig>,
    query: Query<(Entity, Option<&NoWireframe>, Option<&ProcessedForWireframe>), With<Mesh3d>>,
    wireframe_query: Query<Entity, With<Wireframe>>,
    mut commands: Commands,
) {
    // 检查一下有多少个可以渲染wireframe的实体
    let mesh_count = query.iter().count();

    // 如果是第一次运行，输出一些信息
    static FIRST_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    if FIRST_RUN.swap(false, std::sync::atomic::Ordering::Relaxed) {
        info!(
            "线框切换系统已启动, 检测到 {} 个带有Mesh3d的实体",
            mesh_count
        );
    }

    // 移除自动添加Wireframe组件的代码，只在用户按键时添加

    // 按Z键切换全局线框模式
    if keyboard_input.just_pressed(KeyCode::KeyZ) {
        config.global = !config.global;
        info!(
            "切换全局线框模式: {}",
            if config.global { "开启" } else { "关闭" }
        );
    }

    // 按X键切换线框颜色
    if keyboard_input.just_pressed(KeyCode::KeyX) {
        config.default_color = if config.default_color == WHITE.into() {
            info!("切换线框颜色: 粉色");
            DEEP_PINK.into()
        } else {
            info!("切换线框颜色: 白色");
            WHITE.into()
        };

        // 更新所有已经有Wireframe组件的实体的颜色
        for (entity, no_wireframe, _) in query.iter() {
            // 跳过NoWireframe实体
            if no_wireframe.is_some() {
                continue;
            }

            // 只更新已经有Wireframe组件的实体的颜色
            if wireframe_query.contains(entity) {
                commands.entity(entity).insert(WireframeColor {
                    color: config.default_color,
                });
            }
        }
    }

    // 按C键为所有mesh单独添加/移除Wireframe组件
    if keyboard_input.just_pressed(KeyCode::KeyC) {
        info!("单独切换每个mesh的线框显示, 实体数量: {}", mesh_count);

        for (entity, no_wireframe, _) in query.iter() {
            // 跳过被标记为NoWireframe的实体
            if no_wireframe.is_some() {
                continue;
            }

            // 检查实体是否有Wireframe组件
            let entity_commands = commands.get_entity(entity);
            if let Some(mut entity_commands) = entity_commands {
                // 由于我们无法直接检查组件，我们使用一种替代方法来切换
                // 首先移除组件，然后再添加，或者反过来
                entity_commands.remove::<Wireframe>();
                entity_commands.remove::<WireframeColor>();

                // 添加新的组件
                entity_commands.insert(Wireframe);
                entity_commands.insert(WireframeColor {
                    color: config.default_color,
                });

                info!("切换实体 {:?} 的Wireframe组件", entity);
            }
        }
    }
}
