use super::{GeometryData, QuadraticEdge, QuadraticTriangle, VtkError};
use crate::mesh::color_maps::{ColorMapper, ColorMappingConfig};
use crate::mesh::triangulation;
use bevy::prelude::*;
use bevy::utils::HashMap;
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
        ColorMapper::apply_point_color_scalars(self, mesh)
    }

    /// Apply cell color scalars to a mesh.
    ///
    /// This function will try to find a `ColorScalar` attribute in the `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the color scalars are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `ColorScalar`.
    pub fn apply_cell_color_scalars(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        ColorMapper::apply_cell_color_scalars(self, mesh)
    }

    /// Apply scalar attributes to a mesh.
    ///
    /// This function will try to find a `Scalar` attribute in the `Point` or `Cell` location,
    /// and apply it to the mesh as a vertex attribute.
    ///
    /// Returns `Ok(())` if the scalar attributes are successfully applied,
    /// or `Err(VtkError)` if there is no such attribute or if the attribute is not a `Scalar`.
    pub fn apply_scalar_attributes(&self, mesh: &mut Mesh) -> Result<(), VtkError> {
        // Use default color mapping configuration
        let config = ColorMappingConfig::default();
        ColorMapper::apply_scalar_attributes_with_color_map(self, mesh, &config)
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
