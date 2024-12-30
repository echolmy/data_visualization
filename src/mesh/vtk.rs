use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use std::path::PathBuf;
use vtkio::model::{CellType, Piece, UnstructuredGridPiece};
use vtkio::*;

pub fn load_vtk(path: &PathBuf) -> Vtk {
    let vtk_path = PathBuf::from(format!("{}", path.to_string_lossy()));
    if let Ok(vtk_file) = Vtk::import(&vtk_path) {
        vtk_file
    } else {
        panic!("Failed to load VTK file: {}", vtk_path.display());
    }
}
pub fn process_vtk_mesh_legacy(vtk: &Vtk) -> Option<Mesh> {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );

    match &vtk.data {
        // 1. UnstructuredGrid
        model::DataSet::UnstructuredGrid { meta: _, pieces } => {
            _process_legacy_unstructured_grid(&mut mesh, pieces);
            Some(mesh)
        }
        _ => {
            println!("Unsupported Other DataSet. Please select UnstructuredGrid");
            None
        }
    }
}

fn _process_legacy_unstructured_grid(mesh: &mut Mesh, pieces: &[Piece<UnstructuredGridPiece>]) {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let indices: Vec<u32>;
    // legacy should be only one piece::inline

    // 1. process geometry data
    if let Piece::Inline(piece_ptr) = pieces
        .get(0)
        .expect("[Legacy format] Pieces array is empty. Please check")
    {
        // 1.1 process point position
        let points = piece_ptr
            .points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        vertices.reserve(piece_ptr.num_points());
        vertices = points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();

        // 1.2. process point indices to construct triangle
        indices = _process_legacy_unstructuredgrid_cells_triangle(piece_ptr);
    } else {
        panic!("First piece must be an Inline piece")
    }

    // 2. insert data to mesh
    // 2.1. insert points positions
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::from(vertices),
    );
    // 2.2. insert points indices
    mesh.insert_indices(Indices::U32(indices));

    // 2.3 compute normals
    mesh.compute_normals();
}

fn _process_legacy_unstructuredgrid_cells_triangle(
    piece_ptr: &Box<UnstructuredGridPiece>,
) -> Vec<u32> {
    // check every cell type
    for _type in &piece_ptr.cells.types {
        if *_type != CellType::Triangle {
            panic!("cell type contains non-triangle");
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity(piece_ptr.cells.num_cells() * 3);

    // clone operation cannot avoid because of `into_legacy()` operation
    let cell_data = piece_ptr.cells.cell_verts.clone().into_legacy();

    if cell_data.1.len() % 4 != 0 {
        panic!("number of [CELLS] are not multiply of 4.")
    }

    for chunk in cell_data.1.chunks_exact(4) {
        indices.push(chunk[1]);
        indices.push(chunk[2]);
        indices.push(chunk[3]);
    }

    indices
}
