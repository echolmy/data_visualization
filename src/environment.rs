//! # Environment Module
//!
//! This module is responsible for creating and setting up the 3D visualization environment, including:
//! - Grid floor
//! - Coordinate axes (X, Y, Z axes)
//! - Lighting system (key light, fill light, and ambient light)
//!
//! The environment provides a clear 3D reference frame for data visualization.

use bevy::pbr::wireframe::NoWireframe;
use bevy::prelude::*;

/// Size of the grid floor (in world coordinates)
const GRID_SIZE: f32 = 10.0;

/// Number of divisions in the grid floor
const GRID_DIVISIONS: usize = 10;

/// Environment Plugin
///
/// This plugin is responsible for setting up the 3D environment during application startup,
/// including the floor grid, coordinate axes, and lighting system.
///
/// # Example
///
/// ```rust
/// app.add_plugins(EnvironmentPlugin);
/// ```
pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_environment);
    }
}

/// Sets up the 3D environment
///
/// This function is called during application startup and is responsible for creating:
/// - Grid floor as a reference plane
/// - Three coordinate axes (red X-axis, green Y-axis, blue Z-axis)
/// - Three-point lighting system (key light, fill light, ambient light)
///
/// # Parameters
///
/// * `commands` - Bevy's command system for spawning entities
/// * `meshes` - Mesh asset manager
/// * `materials` - Material asset manager
fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create a grid floor
    let grid_mesh = create_grid_mesh(GRID_SIZE, GRID_DIVISIONS);
    commands.spawn((
        Mesh3d(meshes.add(grid_mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.2),
            perceptual_roughness: 1.0,
            metallic: 0.0,
            reflectance: 0.1,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        NoWireframe,
    ));

    // Add coordinate axes
    spawn_axis(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::X,
        Color::srgb(1.0, 0.0, 0.0),
        0.2,
    );
    spawn_axis(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::Y,
        Color::srgb(0.0, 1.0, 0.0),
        0.2,
    );
    spawn_axis(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::Z,
        Color::srgb(0.0, 0.0, 1.0),
        0.2,
    );

    // Key light (main directional light)
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.95, 0.9),
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Fill light (softer, from opposite side)
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: false,
            color: Color::srgb(0.8, 0.85, 1.0),
            ..default()
        },
        Transform::from_xyz(-4.0, 2.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.6, 0.6, 0.7),
        brightness: 0.7,
    });
}

/// Creates mesh data for the grid floor
///
/// Generates a grid composed of lines that serves as a reference floor for 3D space.
/// The grid is positioned on the XZ plane at Y=0.
///
/// # Parameters
///
/// * `size` - Total size of the grid (square side length)
/// * `divisions` - Number of grid divisions, determines the density of grid lines
///
/// # Returns
///
/// Returns a Bevy [`Mesh`] containing vertex, normal, UV coordinate, and index data for the grid lines.
/// The mesh uses `LineList` topology for rendering.
///
/// # Example
///
/// ```rust
/// let grid_mesh = create_grid_mesh(10.0, 10);
/// ```
fn create_grid_mesh(size: f32, divisions: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step = size / divisions as f32;
    let half_size = size / 2.0;

    // Create grid lines along X and Z axes
    for i in 0..=divisions {
        let pos = -half_size + i as f32 * step;

        // Line along X axis
        positions.push([pos, 0.0, -half_size]);
        positions.push([pos, 0.0, half_size]);

        // Line along Z axis
        positions.push([-half_size, 0.0, pos]);
        positions.push([half_size, 0.0, pos]);

        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);

        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 0.0]);
        uvs.push([0.0, 0.0]);
        uvs.push([1.0, 0.0]);

        let base_idx = (i * 4) as u32;
        indices.push(base_idx);
        indices.push(base_idx + 1);
        indices.push(base_idx + 2);
        indices.push(base_idx + 3);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// Spawns a coordinate axis
///
/// Creates a cylindrical coordinate axis to indicate direction in 3D space.
///
/// # Parameters
///
/// * `commands` - Mutable reference to Bevy's command system
/// * `meshes` - Mutable reference to mesh asset manager
/// * `materials` - Mutable reference to material asset manager
/// * `direction` - Direction vector of the axis (Vec3::X, Vec3::Y, or Vec3::Z)
/// * `color` - Color of the axis
/// * `length` - Length of the axis
///
/// # Example
///
/// ```rust
/// // Create a red X-axis
/// spawn_axis(&mut commands, &mut meshes, &mut materials,
///           Vec3::X, Color::srgb(1.0, 0.0, 0.0), 0.2);
/// ```
fn spawn_axis(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    direction: Vec3,
    color: Color,
    length: f32,
) {
    let mesh = meshes.add(Mesh::from(Cylinder {
        radius: 0.01,
        half_height: length / 2.0,
        ..default()
    }));

    let material = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.5,
        ..default()
    });

    let rotation = if direction.abs_diff_eq(Vec3::Y, 0.001) {
        Quat::IDENTITY
    } else if direction.abs_diff_eq(Vec3::X, 0.001) {
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
    } else {
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
    };

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(direction * (length / 2.0)).with_rotation(rotation),
        NoWireframe,
    ));
}
