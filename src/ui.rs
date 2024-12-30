mod events;

use crate::mesh::vtk;
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::*;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<events::OpenFileEvent>()
            .add_event::<events::LoadModelEvent>()
            .add_systems(
                Update,
                (initialize_ui_systems, file_dialog_system, load_resource),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    mut open_file_events: EventWriter<events::OpenFileEvent>,
) {
    egui::TopBottomPanel::top("Menu Bar").show(contexts.ctx_mut(), |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Import").clicked() {
                    // send an event
                    open_file_events.send(events::OpenFileEvent);
                }
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });
}

fn file_dialog_system(
    mut open_events: EventReader<events::OpenFileEvent>,
    mut load_events: EventWriter<events::LoadModelEvent>,
) {
    for _ in open_events.read() {
        if let Some(file) = FileDialog::new()
            .add_filter("model", &["obj", "glb", "vtk"])
            .set_directory("/")
            .pick_file()
        {
            let filepath = PathBuf::from(file.display().to_string());
            println!("open file: {}", filepath.display());
            load_events.send(events::LoadModelEvent(filepath));
        };
    }
}

fn load_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<events::LoadModelEvent>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window_query.get_single().unwrap();
    for events::LoadModelEvent(path) in load_events.read() {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("obj") => {
                commands.spawn((
                    Mesh3d(asset_server.load(format!("{}", path.to_string_lossy()))),
                    Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, -5.0),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.8, 0.7, 0.6),
                        metallic: 0.0,
                        perceptual_roughness: 0.5,
                        ..default()
                    })),
                ));
            }
            // Some("gltf") | Some("glb") => {
            //     commands.spawn((
            //         SceneRoot(asset_server.load(format!("{}#Scene0", path.to_string_lossy()))),
            //         Transform::from_xyz(0.0, 0.0, 0.0),
            //         Visibility::Visible,
            //     ));
            // }
            // legacy format
            Some("vtk") => {
                // let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
                // let vtk_file = Vtk::import(&vtk_path)
                //     .unwrap_or_else(|_| panic!("Failed to load file: {:?}", &vtk_path));
                let vtk_file = vtk::load_vtk(path);
                // match enum vtk.data
                if let Some(mesh) = vtk::process_vtk_mesh_legacy(vtk_file) {
                    commands.spawn((
                        Mesh3d(meshes.add(mesh.clone())),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.7, 0.7, 0.7),
                            metallic: 0.0,
                            perceptual_roughness: 0.5,
                            reflectance: 0.1,
                            ..default()
                        })),
                        Transform::from_xyz(window.width() / 2.0, window.height() / 2.0, -1.0)
                            .with_rotation(Quat::from_euler(
                                EulerRot::XYZ,
                                std::f32::consts::PI / 2.0,
                                std::f32::consts::PI / 4.0,
                                0.0,
                            )),
                        Visibility::Visible,
                    ));

                    // 在spawn后添加
                    println!("Spawned mesh with vertices: {:?}", mesh.count_vertices());
                    // TODO: Check vertices correct or not
                }
            }
            _ => println!("do not support other formats now. Please choose another model."),
        };
    }
}
