use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::utils::HashMap;

use super::{GeometryData, QuadraticEdge, QuadraticTriangle, VtkError};
use crate::mesh::color_maps;
use crate::mesh::triangulation;
use vtkio::*;

// Definition of type of attributes
#[derive(Debug, Clone)]
pub enum AttributeType {
    /// Scalar attribute
    Scalar {
        num_comp: usize,
        table_name: String,
        data: Vec<f32>,
        lookup_table: Option<Vec<[f32; 4]>>,
    },
    /// Color scalar attribute
    ColorScalar { nvalues: u32, data: Vec<Vec<f32>> },
    /// Vector attribute
    Vector(Vec<[f32; 3]>),
    /// Tensor attribute - reserved for future extension
    #[allow(dead_code)]
    Tensor(Vec<[f32; 9]>), // 3x3 tensor matrix
}

/// VTK attribute location definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeLocation {
    Point, // Point attribute
    Cell,  // Cell attribute
}

pub struct UnstructuredGridExtractor;
pub struct PolyDataExtractor;

// Core implementation of GeometryData is in mesh.rs
// Here only provides VTK format specific extension methods
impl GeometryData {
    /// Apply point color scalars to a mesh.
    ///
    /// This function will try to find a `ColorScalar` attribute in the `Point` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the color scalars are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `ColorScalar`.
    pub fn apply_point_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    AttributeLocation::Point => {
                        if let AttributeType::ColorScalar {
                            nvalues: point_nvalues,
                            data: point_data,
                        } = attr
                        {
                            Some((point_nvalues, point_data))
                        } else {
                            None
                        }
                    }
                    _ => None,
                });

            if let Some((nvalues, data)) = color_scalar {
                let colors = self.process_point_color_scalars(*nvalues, data)?;
                if !colors.is_empty() {
                    mesh.insert_attribute(
                        Mesh::ATTRIBUTE_COLOR,
                        VertexAttributeValues::from(colors),
                    );
                    println!("Point color scalars inserted into mesh.");
                    return Ok(());
                }
            }
        }

        println!("No point color attribute found.");
        Ok(())
    }

    /// Process point color scalars from attribute data.
    ///
    /// This function takes the number of values and the data for each point,
    /// and returns a vector of colors.
    ///
    /// Returns `Ok(colors)` if the colors are successfully processed,
    /// or `Err(VtkError)` if the data is invalid.
    fn process_point_color_scalars(
        &self,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Result<Vec<[f32; 4]>, VtkError> {
        if data.len() != self.vertices.len() {
            println!(
                "Warning: color data number({}) does not match vertex number({})",
                data.len(),
                self.vertices.len()
            );
            // TODO: could return error or continue processing
        }

        let mut colors = Vec::with_capacity(self.vertices.len());

        for (idx, color_data) in data.iter().enumerate() {
            if idx >= self.vertices.len() {
                break;
            }

            // match color data number (RGBA or RGB)
            let color = match nvalues {
                3 => [color_data[0], color_data[1], color_data[2], 1.0],
                4 => [color_data[0], color_data[1], color_data[2], color_data[3]],
                _ => [1.0, 1.0, 1.0, 1.0], // default white
            };

            colors.push(color);
        }

        // if color data is not enough, use default color to fill
        if colors.len() < self.vertices.len() {
            colors.resize(self.vertices.len(), [1.0, 1.0, 1.0, 1.0]);
        }

        Ok(colors)
    }

    /// Apply cell color scalars to a mesh.
    ///
    /// This function will try to find a `ColorScalar` attribute in the `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the color scalars are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `ColorScalar`.
    pub fn apply_cell_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        println!("Attributes status: {:?}", self.attributes.is_some());
        if let Some(attributes) = &self.attributes {
            let color_scalar = attributes
                .iter()
                .find_map(|((_, location), attr)| match location {
                    // Get `ColorScalar` data in `Cell` (Only one pair in HashMap).
                    AttributeLocation::Cell => {
                        if let AttributeType::ColorScalar {
                            nvalues: cell_nvalues,
                            data: cell_data,
                        } = attr
                        {
                            Some((cell_nvalues, cell_data))
                        }
                        // No `ColorScalar` data in `Cell`
                        else {
                            None
                        }
                    }
                    // this function does not consider `color scalars in `Point`
                    _ => None,
                });

            // Get vertices color
            let vertices_color = if let Some((nvalues, data)) = color_scalar {
                self.process_cell_color_scalars_internal(*nvalues, data)
            } else {
                Vec::<[f32; 4]>::new()
            };

            // Insert color attributes into Mesh
            if vertices_color.len() != 0 {
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_COLOR,
                    VertexAttributeValues::from(vertices_color),
                );
                println!("Colors inserted into mesh");
            } else {
                println!("No attributes found");
            }
        }
        Ok(())
    }

    /// Process cell color scalars from attribute data.
    ///
    /// This function takes the number of values and the data for each cell,
    /// and returns a vector of colors.
    ///
    /// Returns `Ok(colors)` if the colors are successfully processed,
    /// or `Err(VtkError)` if the data is invalid.
    fn process_cell_color_scalars_internal(
        &self,
        nvalues: u32,
        data: &Vec<Vec<f32>>,
    ) -> Vec<[f32; 4]> {
        // initialize color list for each vertex (white)
        let mut vertices_color = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

        if let Some(mapping) = &self.triangle_to_cell_mapping {
            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                // Check if cell_idx is valid
                if cell_idx >= data.len() {
                    println!(
                        "Warning: cell index {} exceeds color data range {}",
                        cell_idx,
                        data.len()
                    );
                    continue;
                }

                // Get the three vertex indices of this triangle
                let triangle_base = triangle_idx * 3;
                if triangle_base + 2 >= self.indices.len() {
                    println!(
                        "Warning: triangle index {} exceeds index range {}",
                        triangle_base,
                        self.indices.len()
                    );
                    continue;
                }

                let vertex_indices = [
                    self.indices[triangle_base] as usize,
                    self.indices[triangle_base + 1] as usize,
                    self.indices[triangle_base + 2] as usize,
                ];

                // Get the color of the corresponding cell
                let colors = &data[cell_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                };

                // Set vertex colors
                for &idx in &vertex_indices {
                    if idx < vertices_color.len() {
                        vertices_color[idx] = color;
                    }
                }
            }
        } else {
            // Fall back to original method, one-to-one correspondence in order
            let num_triangles = self.indices.len() / 3;
            for triangle_idx in 0..num_triangles {
                if triangle_idx >= data.len() {
                    println!(
                        "Warning: triangle index {} exceeds color data range {}",
                        triangle_idx,
                        data.len()
                    );
                    break;
                }

                // Get the three vertex indices of this triangle
                let vertex_indices = [
                    self.indices[triangle_idx * 3] as usize,
                    self.indices[triangle_idx * 3 + 1] as usize,
                    self.indices[triangle_idx * 3 + 2] as usize,
                ];

                // Get the color of this cell
                let colors = &data[triangle_idx];
                let color = match nvalues {
                    3 => [colors[0], colors[1], colors[2], 1.0],
                    4 => [colors[0], colors[1], colors[2], colors[3]],
                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                };

                // Set vertex colors
                for &idx in &vertex_indices {
                    if idx < vertices_color.len() {
                        vertices_color[idx] = color;
                    }
                }
            }
        }

        vertices_color
    }

    /// Apply scalar attributes to a mesh.
    ///
    /// This function will try to find a `Scalar` attribute in the `Point` or `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the scalar attributes are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `Scalar`.
    pub fn apply_scalar_attributes(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        if let Some(attributes) = &self.attributes {
            // Process point scalars
            for ((name, location), attr) in attributes.iter() {
                if let AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                    lookup_table,
                } = attr
                {
                    if location == &AttributeLocation::Point && *num_comp == 1 {
                        println!(
                            "Processing point scalar attribute: {} lookup table: {}",
                            name, table_name
                        );

                        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

                        // Calculate min/max values for normalization
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        let range = max_val - min_val;

                        println!(
                            "Scalar data range: min={}, max={}, range={}",
                            min_val, max_val, range
                        );

                        // Set color for each vertex
                        for (i, &val) in data.iter().enumerate() {
                            if i < self.vertices.len() {
                                let color = if range < 1e-10 {
                                    if i == 0 {
                                        println!("Point data range is very small ({}), using middle color for constant value {} ", range, val);
                                    }
                                    if let Some(lut) = lookup_table {
                                        let temp_color_map = color_maps::ColorMap {
                                            name: table_name.clone(),
                                            colors: lut.clone(),
                                        };
                                        temp_color_map.get_interpolated_color(0.5)
                                    } else {
                                        // Use default color mapping's middle color
                                        let color_map = color_maps::get_color_map(table_name);
                                        color_map.get_interpolated_color(0.5)
                                    }
                                } else {
                                    let normalized = (val - min_val) / range;

                                    // Use ColorMap to get color
                                    if let Some(lut) = lookup_table {
                                        let temp_color_map = color_maps::ColorMap {
                                            name: table_name.clone(),
                                            colors: lut.clone(),
                                        };
                                        temp_color_map.get_interpolated_color(normalized)
                                    } else {
                                        let color_map = color_maps::get_color_map(table_name);
                                        color_map.get_interpolated_color(normalized)
                                    }
                                };

                                // Print debug info
                                if i < 10 {
                                    // Only print first 10 vertices to avoid too much output
                                    println!("Vertex {}: scalar value = {}, range = {}, color = [{:.3}, {:.3}, {:.3}, {:.3}]",
                                        i, val, range, color[0], color[1], color[2], color[3]);
                                }

                                vertex_colors[i] = color;
                            }
                        }

                        // Insert color attribute
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            VertexAttributeValues::from(vertex_colors),
                        );
                        println!("Point scalar colors inserted into mesh");
                        return Ok(());
                    }
                }
            }

            // If no point scalars, then process cell scalars
            for ((name, location), attr) in attributes.iter() {
                if let AttributeType::ColorScalar { nvalues, data } = attr {
                    if location == &AttributeLocation::Cell {
                        println!(
                            "Processing cell color scalar attribute: {} (nvalues: {})",
                            name, nvalues
                        );
                        println!("Color data: {:?}", data);

                        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

                        // Use mapping relationship
                        if let Some(mapping) = &self.triangle_to_cell_mapping {
                            println!("Using triangle to cell mapping");
                            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                                // Check if cell_idx is valid
                                if cell_idx >= data.len() {
                                    println!(
                                        "Warning: cell index {} exceeds color data range {}",
                                        cell_idx,
                                        data.len()
                                    );
                                    continue;
                                }

                                // Get the three vertex indices of this triangle
                                let triangle_base = triangle_idx * 3;
                                if triangle_base + 2 >= self.indices.len() {
                                    println!(
                                        "Warning: triangle index {} exceeds index range {}",
                                        triangle_base,
                                        self.indices.len()
                                    );
                                    continue;
                                }

                                let vertex_indices = [
                                    self.indices[triangle_base] as usize,
                                    self.indices[triangle_base + 1] as usize,
                                    self.indices[triangle_base + 2] as usize,
                                ];

                                // Get the color of the corresponding cell
                                let colors = &data[cell_idx];
                                let color = match nvalues {
                                    3 => [colors[0], colors[1], colors[2], 1.0],
                                    4 => [colors[0], colors[1], colors[2], colors[3]],
                                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                                };

                                println!(
                                    "Triangle {}, Cell {}: color = [{:.2}, {:.2}, {:.2}, {:.2}]",
                                    triangle_idx, cell_idx, color[0], color[1], color[2], color[3]
                                );

                                // Set vertex colors
                                for &idx in &vertex_indices {
                                    if idx < vertex_colors.len() {
                                        vertex_colors[idx] = color;
                                    }
                                }
                            }
                        } else {
                            println!("No triangle to cell mapping, using default mapping");
                            let num_triangles = self.indices.len() / 3;
                            for triangle_idx in 0..num_triangles {
                                if triangle_idx >= data.len() {
                                    println!(
                                        "Warning: triangle index {} exceeds color data range {}",
                                        triangle_idx,
                                        data.len()
                                    );
                                    break;
                                }

                                // Get the three vertex indices of this triangle
                                let vertex_indices = [
                                    self.indices[triangle_idx * 3] as usize,
                                    self.indices[triangle_idx * 3 + 1] as usize,
                                    self.indices[triangle_idx * 3 + 2] as usize,
                                ];

                                // Get the color of this cell
                                let colors = &data[triangle_idx];
                                let color = match nvalues {
                                    3 => [colors[0], colors[1], colors[2], 1.0],
                                    4 => [colors[0], colors[1], colors[2], colors[3]],
                                    _ => [1.0, 1.0, 1.0, 1.0], // default color white
                                };

                                println!(
                                    "Triangle {}: color = [{:.2}, {:.2}, {:.2}, {:.2}]",
                                    triangle_idx, color[0], color[1], color[2], color[3]
                                );

                                // Set vertex colors
                                for &idx in &vertex_indices {
                                    if idx < vertex_colors.len() {
                                        vertex_colors[idx] = color;
                                    }
                                }
                            }
                        }

                        // Insert color attribute
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            VertexAttributeValues::from(vertex_colors),
                        );
                        println!("Cell color scalar inserted into mesh");
                        return Ok(());
                    }
                }

                if let AttributeType::Scalar {
                    num_comp,
                    table_name,
                    data,
                    lookup_table,
                } = attr
                {
                    if location == &AttributeLocation::Cell && *num_comp == 1 {
                        println!(
                            "Processing cell scalar attribute: {} lookup table: {}",
                            name, table_name
                        );

                        let mut vertex_colors = vec![[1.0, 1.0, 1.0, 1.0]; self.vertices.len()];

                        // Calculate min/max values for normalization
                        let min_val = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        let range = max_val - min_val;

                        // Use mapping relationship
                        if let Some(mapping) = &self.triangle_to_cell_mapping {
                            for (triangle_idx, &cell_idx) in mapping.iter().enumerate() {
                                // Check if cell_idx is valid
                                if cell_idx >= data.len() {
                                    println!(
                                        "Warning: cell index {} exceeds scalar data range {}",
                                        cell_idx,
                                        data.len()
                                    );
                                    continue;
                                }

                                // Get cell scalar value and calculate color
                                let val = data[cell_idx];
                                let color = if range < 1e-10 {
                                    if triangle_idx == 0 {
                                        println!("Cell data range is very small ({}), using middle color for constant value {}", range, val);
                                    }
                                    if let Some(lut) = lookup_table {
                                        let temp_color_map = color_maps::ColorMap {
                                            name: table_name.clone(),
                                            colors: lut.clone(),
                                        };
                                        temp_color_map.get_interpolated_color(0.5)
                                    } else {
                                        // Use default color mapping's middle color
                                        let color_map = color_maps::get_color_map(table_name);
                                        color_map.get_interpolated_color(0.5)
                                    }
                                } else {
                                    let normalized = (val - min_val) / range;

                                    // Use ColorMap to get color
                                    if let Some(lut) = lookup_table {
                                        let temp_color_map = color_maps::ColorMap {
                                            name: table_name.clone(),
                                            colors: lut.clone(),
                                        };
                                        temp_color_map.get_interpolated_color(normalized)
                                    } else {
                                        let color_map = color_maps::get_color_map(table_name);
                                        color_map.get_interpolated_color(normalized)
                                    }
                                };

                                // Get the three vertex indices of this triangle
                                let triangle_base = triangle_idx * 3;
                                if triangle_base + 2 >= self.indices.len() {
                                    println!(
                                        "Warning: triangle index {} exceeds index range {}",
                                        triangle_base,
                                        self.indices.len()
                                    );
                                    continue;
                                }

                                let vertex_indices = [
                                    self.indices[triangle_base] as usize,
                                    self.indices[triangle_base + 1] as usize,
                                    self.indices[triangle_base + 2] as usize,
                                ];

                                // Set vertex colors
                                for &idx in &vertex_indices {
                                    if idx < vertex_colors.len() {
                                        vertex_colors[idx] = color;
                                    }
                                }

                                // Print debug info
                                if triangle_idx < 10 {
                                    // Only print first 10 triangles to avoid too much output
                                    println!("Triangle {}, Cell {}: scalar value = {}, range = {}, color = [{:.3}, {:.3}, {:.3}, {:.3}]",
                                        triangle_idx, cell_idx, val, range, color[0], color[1], color[2], color[3]);
                                }
                            }
                        }

                        // Insert color attribute
                        mesh.insert_attribute(
                            Mesh::ATTRIBUTE_COLOR,
                            VertexAttributeValues::from(vertex_colors),
                        );
                        println!("Cell scalar colors inserted into mesh");
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    // Extract all lookup tables from attributes
    pub fn extract_lookup_tables(&mut self) {
        if let Some(attributes) = &self.attributes {
            for ((name, _), attr) in attributes.iter() {
                if name.starts_with("__lut_") {
                    if let AttributeType::Scalar {
                        table_name,
                        lookup_table: Some(colors),
                        ..
                    } = attr
                    {
                        self.lookup_tables
                            .insert(table_name.clone(), colors.clone());
                    }
                }
            }
        }
    }

    // Get lookup table with specified name
    // pub fn get_lookup_table(&self, name: &str) -> Option<&Vec<[f32; 4]>> {
    //     self.lookup_tables.get(name)
    // }

    // Check if lookup table with specified name exists
    // pub fn has_lookup_table(&self, name: &str) -> bool {
    //     self.lookup_tables.contains_key(name)
    // }

    // Get names of all lookup tables
    // pub fn get_lookup_table_names(&self) -> Vec<String> {
    //     self.lookup_tables.keys().cloned().collect()
    // }
}

pub trait VtkMeshExtractor {
    // associated type
    type PieceType;

    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError>;

    // basic geometry process
    fn extract_vertices(&self, points: &IOBuffer) -> Vec<[f32; 3]> {
        // process point position
        let points = points
            .cast_into::<f32>()
            .expect("IOBuffer converted failed.");
        // construct position of each vertex
        points.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect()
    }

    #[allow(dead_code)]
    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32>;

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError>;

    fn process_data_array(
        &self,
        name: &str,
        elem_type: &model::ElementType,
        data: &IOBuffer,
    ) -> Result<(String, AttributeType), VtkError>;
}

impl UnstructuredGridExtractor {
    // general triangulate cells - Use general triangulation function, returns quadratic triangle and quadratic edge data
    fn triangulate_cells(
        &self,
        cells: model::Cells,
    ) -> (
        Vec<u32>,
        Vec<usize>,
        Vec<QuadraticTriangle>,
        Vec<QuadraticEdge>,
    ) {
        triangulation::triangulate_cells(cells)
    }
}

impl VtkMeshExtractor for UnstructuredGridExtractor {
    type PieceType = Vec<model::Piece<model::UnstructuredGridPiece>>;

    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
        let mut attributes = HashMap::new();
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found"))?;

        if let model::Piece::Inline(piece) = piece {
            // Process point data attributes
            for point_data in &piece.data.point {
                match point_data {
                    model::Attribute::DataArray(array) => {
                        if let Ok((name, attr)) =
                            self.process_data_array(&array.name, &array.elem, &array.data)
                        {
                            attributes.insert((name, AttributeLocation::Point), attr);
                        }
                    }
                    _ => println!("Unsupported attribute type"),
                }
            }

            // Process cell data attributes
            for cell_data in &piece.data.cell {
                match cell_data {
                    model::Attribute::DataArray(array) => {
                        if let Ok((name, attr)) =
                            self.process_data_array(&array.name, &array.elem, &array.data)
                        {
                            attributes.insert((name, AttributeLocation::Cell), attr);
                        }
                    }
                    _ => println!("Unsupported attribute type"),
                }
            }
        }

        Ok(attributes)
    }

    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32> {
        if let Some(model::Piece::Inline(piece)) = pieces.into_iter().next() {
            let (indices, _, _, _) = self.triangulate_cells(piece.cells);
            indices
        } else {
            Vec::new()
        }
    }

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found"))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data"));
        };

        let vertices = self.extract_vertices(&piece.points);
        let (indices, triangle_to_cell_mapping, quadratic_triangles, quadratic_edges) =
            self.triangulate_cells(piece.cells.clone());
        let attributes = self.extract_attributes_legacy(&pieces)?;

        let mut geometry = GeometryData::new(vertices, indices, attributes);
        geometry.extract_lookup_tables();
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);

        // Add quadratic triangle data (if any)
        if !quadratic_triangles.is_empty() {
            geometry = geometry.add_quadratic_triangles(quadratic_triangles);
        }

        // Add quadratic edge data (if any)
        if !quadratic_edges.is_empty() {
            geometry = geometry.add_quadratic_edges(quadratic_edges);
        }

        Ok(geometry)
    }

    fn process_data_array(
        &self,
        name: &str,
        elem_type: &model::ElementType,
        data: &IOBuffer,
    ) -> Result<(String, AttributeType), VtkError> {
        let values = data.cast_into::<f32>().unwrap();

        match elem_type {
            model::ElementType::Scalars {
                num_comp,
                lookup_table,
            } => {
                println!(
                    "Processing scalar data: {} components, lookup table: {:?}",
                    num_comp, lookup_table
                );

                Ok((
                    name.to_string(),
                    AttributeType::Scalar {
                        num_comp: *num_comp as usize,
                        table_name: lookup_table
                            .clone()
                            .unwrap_or_else(|| "default".to_string()),
                        data: values,
                        lookup_table: None,
                    },
                ))
            }
            model::ElementType::ColorScalars(nvalues) => {
                let color_values = values
                    .chunks_exact(*nvalues as usize)
                    .map(|v| v.to_vec())
                    .collect();

                Ok((
                    name.to_string(),
                    AttributeType::ColorScalar {
                        nvalues: *nvalues,
                        data: color_values,
                    },
                ))
            }
            model::ElementType::Vectors => {
                // Process vector type, each vector has 3 components (x,y,z)
                let vectors: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(vectors)))
            }
            model::ElementType::Normals => {
                // Process normal vectors, similar to Vectors
                let normals: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(normals)))
            }
            model::ElementType::TCoords(n) => {
                println!("Texture coordinates: {} components", n);
                // Simple processing as vector type
                let coords: Vec<[f32; 3]> = if *n == 2 {
                    // 2D texture coordinates, third component is 0
                    values
                        .chunks_exact(2)
                        .map(|chunk| [chunk[0], chunk[1], 0.0])
                        .collect()
                } else if *n == 3 {
                    // 3D texture coordinates
                    values
                        .chunks_exact(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                        .collect()
                } else {
                    return Err(VtkError::InvalidFormat(
                        "Unsupported texture coordinate dimension",
                    ));
                };

                Ok((name.to_string(), AttributeType::Vector(coords)))
            }
            model::ElementType::Tensors => {
                println!("Tensor data is not fully supported, simplified processing");
                // Simplified processing as vector collection
                let tensors: Vec<[f32; 3]> = values
                    .chunks_exact(9) // 3x3 tensor
                    .map(|chunk| {
                        // Simplified: use diagonal elements
                        [chunk[0], chunk[4], chunk[8]]
                    })
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(tensors)))
            }
            model::ElementType::LookupTable => {
                // Convert the lookup table data into RGBA colors
                let colors: Vec<[f32; 4]> = values
                    .chunks_exact(4)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
                    .collect();

                println!(
                    "Processed lookup table {} with {} colors",
                    name,
                    colors.len()
                );

                // Return lookup table as a special scalar attribute
                Ok((
                    format!("__lut_{}", name), // Use special prefix to mark lookup table
                    AttributeType::Scalar {
                        num_comp: 4, // RGBA
                        table_name: name.to_string(),
                        data: values,
                        lookup_table: Some(colors),
                    },
                ))
            }
            _ => {
                println!("Unsupported data type");
                Err(VtkError::UnsupportedDataType)
            }
        }
    }
}

impl VtkMeshExtractor for PolyDataExtractor {
    type PieceType = Vec<model::Piece<model::PolyDataPiece>>;

    fn extract_attributes_legacy(
        &self,
        pieces: &Self::PieceType,
    ) -> Result<HashMap<(String, AttributeLocation), AttributeType>, VtkError> {
        let mut attributes = HashMap::new();

        let model::Piece::Inline(piece) = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found"))?
        else {
            return Err(VtkError::InvalidFormat("Expected inline data"));
        };

        let point_attr_list = &piece.data.point;
        let cell_attr_list = &piece.data.cell;

        // Process point attributes
        if !point_attr_list.is_empty() {
            for point_attr in point_attr_list {
                match point_attr {
                    model::Attribute::DataArray(data_array) => {
                        if let Ok((name, attr_type)) = self.process_data_array(
                            &data_array.name,
                            &data_array.elem,
                            &data_array.data,
                        ) {
                            attributes.insert((name, AttributeLocation::Point), attr_type);
                        }
                    }
                    _ => println!("Unsupported attribute type"),
                }
            }
        }

        // Process cell attributes
        if !cell_attr_list.is_empty() {
            for cell_attr in cell_attr_list {
                match cell_attr {
                    model::Attribute::DataArray(data_array) => {
                        if let Ok((name, attr_type)) = self.process_data_array(
                            &data_array.name,
                            &data_array.elem,
                            &data_array.data,
                        ) {
                            attributes.insert((name, AttributeLocation::Cell), attr_type);
                        }
                    }
                    _ => println!("Unsupported attribute type"),
                }
            }
        }

        Ok(attributes)
    }

    fn extract_indices(&self, pieces: Self::PieceType) -> Vec<u32> {
        if let Ok((indices, _)) = self.process_polydata(pieces) {
            indices
        } else {
            Vec::new()
        }
    }

    fn process_legacy(&self, pieces: Self::PieceType) -> Result<GeometryData, VtkError> {
        let piece = pieces
            .first()
            .ok_or(VtkError::MissingData("No pieces found".into()))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Expected inline data".into()));
        };

        let attributes = self.extract_attributes_legacy(&pieces)?;
        let vertices = self.extract_vertices(&piece.points);
        let (indices, triangle_to_cell_mapping) = self.process_polydata(pieces.clone())?;

        let mut geometry = GeometryData::new(vertices, indices, attributes);
        geometry.extract_lookup_tables(); // Extract lookup tables
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);

        Ok(geometry)
    }

    fn process_data_array(
        &self,
        name: &str,
        elem_type: &model::ElementType,
        data: &IOBuffer,
    ) -> Result<(String, AttributeType), VtkError> {
        let values = data.cast_into::<f32>().unwrap();

        match elem_type {
            model::ElementType::Scalars {
                num_comp,
                lookup_table,
            } => {
                println!(
                    "Processing scalar data: {} components, lookup table: {:?}",
                    num_comp, lookup_table
                );

                Ok((
                    name.to_string(),
                    AttributeType::Scalar {
                        num_comp: *num_comp as usize,
                        table_name: lookup_table
                            .clone()
                            .unwrap_or_else(|| "default".to_string()),
                        data: values,
                        lookup_table: None,
                    },
                ))
            }
            model::ElementType::ColorScalars(nvalues) => {
                let color_values = values
                    .chunks_exact(*nvalues as usize)
                    .map(|v| v.to_vec())
                    .collect();

                Ok((
                    name.to_string(),
                    AttributeType::ColorScalar {
                        nvalues: *nvalues,
                        data: color_values,
                    },
                ))
            }
            model::ElementType::Vectors => {
                let vectors: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(vectors)))
            }
            model::ElementType::Normals => {
                let normals: Vec<[f32; 3]> = values
                    .chunks_exact(3)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(normals)))
            }
            model::ElementType::TCoords(n) => {
                println!("Texture coordinates: {} components", n);
                let coords: Vec<[f32; 3]> = if *n == 2 {
                    values
                        .chunks_exact(2)
                        .map(|chunk| [chunk[0], chunk[1], 0.0])
                        .collect()
                } else if *n == 3 {
                    values
                        .chunks_exact(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2]])
                        .collect()
                } else {
                    return Err(VtkError::InvalidFormat(
                        "Unsupported texture coordinate dimension",
                    ));
                };

                Ok((name.to_string(), AttributeType::Vector(coords)))
            }
            model::ElementType::Tensors => {
                println!("Tensor data is not fully supported, simplified processing");
                let tensors: Vec<[f32; 3]> = values
                    .chunks_exact(9)
                    .map(|chunk| [chunk[0], chunk[4], chunk[8]])
                    .collect();

                Ok((name.to_string(), AttributeType::Vector(tensors)))
            }
            model::ElementType::LookupTable => {
                let colors: Vec<[f32; 4]> = values
                    .chunks_exact(4)
                    .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
                    .collect();

                println!(
                    "Processed lookup table {} with {} colors",
                    name,
                    colors.len()
                );

                Ok((
                    format!("__lut_{}", name),
                    AttributeType::Scalar {
                        num_comp: 4,
                        table_name: name.to_string(),
                        data: values,
                        lookup_table: Some(colors),
                    },
                ))
            }
            _ => {
                println!("Unsupported data type");
                Err(VtkError::UnsupportedDataType)
            }
        }
    }
}

impl PolyDataExtractor {
    // Improved polygon data processing method
    fn process_polydata(
        &self,
        pieces: Vec<model::Piece<model::PolyDataPiece>>,
    ) -> Result<(Vec<u32>, Vec<usize>), VtkError> {
        let piece = pieces
            .into_iter()
            .next()
            .ok_or(VtkError::MissingData("Fragment not found"))?;
        let model::Piece::Inline(piece) = piece else {
            return Err(VtkError::InvalidFormat("Inline data required"));
        };

        let mut indices = Vec::<u32>::new();
        let mut triangle_to_cell_mapping = Vec::<usize>::new();

        // Process vertex topology (skip, because they don't form a surface)
        if let Some(_) = piece.verts {
            println!("find verts - skip, because they don't form a surface");
        }

        // Process line topology (skip, because they don't form a surface)
        if let Some(_) = piece.lines {
            println!("find lines - skip, because they don't form a surface");
        }

        // Process polygon topology - main processing logic
        if let Some(polys) = piece.polys {
            let (polys_indices, polys_mapping) = self.triangulate_polygon(polys);
            indices.extend(polys_indices);
            triangle_to_cell_mapping.extend(polys_mapping);
        }

        // Process triangle strips (not implemented yet)
        if let Some(_strips) = piece.strips {
            println!("find strips - not supported");
            // todo!()
        }

        if indices.is_empty() {
            return Err(VtkError::MissingData(
                "No surface geometry found in the fragment",
            ));
        }

        Ok((indices, triangle_to_cell_mapping))
    }

    fn triangulate_polygon(&self, topology: model::VertexNumbers) -> (Vec<u32>, Vec<usize>) {
        // Use function from general triangulation module
        triangulation::triangulate_polygon(topology)
    }
}
