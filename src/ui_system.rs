use bevy::prelude::*;
use bevy_egui::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use rfd::FileDialog;
use std::path::PathBuf;
use vtkio::model::{CellType, Cells, VertexNumbers};
use vtkio::*;

#[derive(Event)]
pub struct OpenFileEvent;

#[derive(Event)]
pub struct LoadModelEvent(PathBuf);

#[derive(Component)]
pub struct Model;

// // TODO: change to the other file
// #[derive(Debug)]
// struct MeshData {
//     // geometry data
//     vertices: Vec<[f32; 3]>,
//     triangles: Vec<[u32; 3]>,
//     Tetra: Vec<[u32; 4]>,
// }
//
// impl MeshData {
//     fn new() -> Self {
//         Self {
//             vertices: Vec::new(),
//             triangles: Vec::new(),
//             Tetra: Vec::new(),
//         }
//     }
// }

pub struct UIPlugin;
impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<OpenFileEvent>()
            .add_event::<LoadModelEvent>()
            .add_systems(
                Update,
                (initialize_ui_systems, file_dialog_system, load_resource),
            );
        // .add_plugins(ObjPlugin);
    }
}

fn initialize_ui_systems(
    mut contexts: EguiContexts,
    mut open_file_events: EventWriter<OpenFileEvent>,
) {
    egui::TopBottomPanel::top("Menu Bar").show(contexts.ctx_mut(), |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Import").clicked() {
                    // send an event
                    open_file_events.send(OpenFileEvent);
                }
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });
}

fn file_dialog_system(
    mut open_events: EventReader<OpenFileEvent>,
    mut load_events: EventWriter<LoadModelEvent>,
) {
    for _ in open_events.read() {
        if let Some(file) = FileDialog::new()
            .add_filter("model", &["obj", "glb", "vtk"])
            .set_directory("/")
            .pick_file()
        {
            let filepath = PathBuf::from(file.display().to_string());
            println!("open file: {}", filepath.display());
            load_events.send(LoadModelEvent(filepath));
        };
    }
}

fn load_resource(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut load_events: EventReader<LoadModelEvent>,
) {
    for LoadModelEvent(path) in load_events.read() {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("obj") => {
                commands.spawn((
                    Mesh3d(asset_server.load(format!("{}", path.to_string_lossy()))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.8, 0.7, 0.6),
                        metallic: 0.0,
                        perceptual_roughness: 0.5,
                        ..default()
                    })),
                ));
            }
            Some("gltf") | Some("glb") => {
                commands.spawn((
                    SceneRoot(asset_server.load(format!("{}#Scene0", path.to_string_lossy()))),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    Visibility::Visible,
                ));
            }
            Some("vtk") => {
                let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
                let vtk_file = Vtk::import(&vtk_path)
                    .unwrap_or_else(|_| panic!("Failed to load file: {:?}", &vtk_path));

                // match enum vtk.data
                if let Some(mesh) = process_vtk_mesh(&vtk_file) {
                    commands.spawn((
                        Mesh3d(meshes.add(mesh.clone())),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.7, 0.7, 0.7),
                            metallic: 0.0,
                            perceptual_roughness: 0.5,
                            reflectance: 0.1,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_euler(
                            EulerRot::XYZ,
                            std::f32::consts::PI / 2.0,
                            std::f32::consts::PI / 4.0,
                            0.0,
                        )),
                        Visibility::Visible,
                        Model,
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

fn process_vtk_mesh(vtk: &Vtk) -> Option<Mesh> {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    match &vtk.data {
        // 1. UnstructuredGrid
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            let mut vertices_all_pieces: Vec<Vec<[f32; 3]>> = Vec::new(); //暂时不用，视情况看以后有了多pieces的数据再看
            let mut vertices: Vec<[f32; 3]> = Vec::new();
            let mut indices_all_pieces: Vec<Vec<u32>> = Vec::new(); //暂时不用，视情况看以后有了多pieces的数据再看
            let mut indices: Vec<u32> = Vec::new();
            for piece in pieces {
                match piece {
                    // 1.1 process vertices data
                    // Piece::Inline
                    model::Piece::Inline(grid_piece) => {
                        let points = grid_piece
                            .points
                            .cast_into::<f32>()
                            .expect("IOBuffer converted failed.");

                        let vertices_per_piece: Vec<[f32; 3]> =
                            points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();

                        vertices_all_pieces.push(vertices_per_piece.clone());
                        vertices.append(&mut vertices_per_piece.clone());

                        // 1.2 process vertices indices
                        let mut indices_per_piece = Vec::new();
                        process_vtk_cells(
                            vertices_per_piece,
                            &mut indices_per_piece,
                            &grid_piece.cells,
                        );
                        
                        indices.append(&mut indices_per_piece);
                        indices_all_pieces.push(indices_per_piece);
                    }

                    // Piece::{Source, Loaded}
                    _ => {
                        println!("Source or Loaded");
                    }
                }
            }

            // 1.2 create mesh
            println!("Finished processing mesh. mesh vertices: {:?}", vertices);
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                VertexAttributeValues::from(vertices.clone()),
            );

            println!("indices:{:?}", indices);
            mesh.insert_indices(Indices::U32(indices.clone()));

            mesh.compute_normals();

            Some(mesh)
        }
        // enum ImageData, StructuredGrid, RectilinearGrid, PolyData, Field
        _ => None,
    }
}

fn process_vtk_cells(
    // TODO: Unused variable
    _vertices_per_piece: Vec<[f32; 3]>,
    indices_per_piece: &mut Vec<u32>,
    cells: &Cells,
) {
    match &cells.cell_verts {
        VertexNumbers::Legacy {
            num_cells: _,
            vertices,
        } => {
            // read data from vertices
            // e.g. vertices = [4, 0, 1, 2, 3]:
            // 4 : number of vertices
            // [0,1,2,3] : index
            let mut current_index = 0;
            for cell_type in &cells.types {
                let n_vertices = vertices[current_index] as usize;
                let vertex_indices =
                    vertices[current_index + 1..current_index + 1 + n_vertices].to_vec();
                match cell_type {
                    CellType::Tetra => {
                        indices_per_piece.extend_from_slice(&[
                            vertex_indices[0],
                            vertex_indices[2],
                            vertex_indices[1], // face 1
                            vertex_indices[0],
                            vertex_indices[3],
                            vertex_indices[2], // face 2
                            vertex_indices[0],
                            vertex_indices[1],
                            vertex_indices[3], // face 3
                            vertex_indices[1],
                            vertex_indices[2],
                            vertex_indices[3], // face 4
                        ]);
                    }
                    _ => println!("TODO: Unsupported cell type: {:?}", cell_type),
                }
                current_index += 1 + n_vertices;
            }
            // println!("{:?}", num_cells);
        }
        VertexNumbers::XML {
            connectivity,
            offsets,
        } => {
            println!("{:?}, {:?}", connectivity, offsets);
        }
    }
}
