//! # LOD (Level of Detail) 系统
//!
//! 自动根据相机距离管理多个细节层级：
//! - LOD0: 原始模型（最高精度）
//! - LOD1: 简化模型（50%三角形）
//! - LOD2: 最简化模型（25%三角形）

use crate::mesh::{GeometryData, VtkError};
use crate::ui::{CurrentModelData, UserModelMesh};
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::collections::BTreeMap;

/// LOD级别定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LODLevel {
    /// 原始精度（最高精度）
    LOD0 = 0,
    /// 简化精度1
    LOD1 = 1,
    /// 最简化精度
    LOD2 = 2,
}

impl LODLevel {
    /// 获取切换距离阈值
    pub fn distance_threshold(self) -> f32 {
        match self {
            LODLevel::LOD0 => 15.0,      // 近距离使用原始精度 
            LODLevel::LOD1 => 30.0,     // 中远距离使用简化精度 
            LODLevel::LOD2 => f32::MAX, // 最远距离使用最简化精度
        }
    }

    /// 获取所有级别的有序列表
    pub fn all_levels() -> Vec<LODLevel> {
        vec![LODLevel::LOD0, LODLevel::LOD1, LODLevel::LOD2]
    }
}

/// LOD网格数据
#[derive(Debug)]
pub struct LODMeshData {
    pub geometry: GeometryData,
    pub mesh_handle: Handle<Mesh>,
    #[allow(dead_code)]
    pub triangle_count: usize,
}

/// LOD管理器组件
#[derive(Component)]
pub struct LODManager {
    /// 存储各个LOD级别的网格数据
    pub lod_meshes: BTreeMap<LODLevel, LODMeshData>,
    /// 当前活跃的LOD级别
    pub current_lod: LODLevel,
    /// 模型的包围盒中心（用于距离计算）
    pub model_center: Vec3,
    /// 模型的包围盒大小（用于距离计算）
    pub model_size: f32,
    /// 是否需要更新LOD
    pub needs_update: bool,
}

impl LODManager {
    /// 创建新的LOD管理器
    pub fn new(
        original_geometry: GeometryData,
        meshes: &mut ResMut<Assets<Mesh>>,
    ) -> Result<Self, VtkError> {
        let mut lod_meshes = BTreeMap::new();
        let triangle_count = original_geometry.indices.len() / 3;

        println!("创建LOD管理器，原始模型有 {} 个三角形", triangle_count);

        // 计算模型包围盒
        let (model_center, model_size) = calculate_bounding_box(&original_geometry.vertices);

        // LOD0: 原始模型（最高精度）
        let original_mesh = crate::mesh::create_mesh_from_geometry(&original_geometry);
        let original_handle = meshes.add(original_mesh);
        lod_meshes.insert(
            LODLevel::LOD0,
            LODMeshData {
                geometry: original_geometry.clone(),
                mesh_handle: original_handle,
                triangle_count,
            },
        );
        println!("LOD0 原始模型完成，{} 个三角形", triangle_count);

        // LOD1: 简化版本（50%三角形）
        if let Ok(simplified_geometry) = simplify_mesh(&original_geometry, 0.5) {
            let simplified_mesh = crate::mesh::create_mesh_from_geometry(&simplified_geometry);
            let simplified_handle = meshes.add(simplified_mesh);
            let simplified_triangle_count = simplified_geometry.indices.len() / 3;

            lod_meshes.insert(
                LODLevel::LOD1,
                LODMeshData {
                    geometry: simplified_geometry,
                    mesh_handle: simplified_handle,
                    triangle_count: simplified_triangle_count,
                },
            );
            println!("LOD1 简化完成，生成 {} 个三角形", simplified_triangle_count);
        }

        // LOD2: 最简化版本（25%三角形）
        if let Ok(most_simplified_geometry) = simplify_mesh(&original_geometry, 0.25) {
            let most_simplified_mesh =
                crate::mesh::create_mesh_from_geometry(&most_simplified_geometry);
            let most_simplified_handle = meshes.add(most_simplified_mesh);
            let most_simplified_triangle_count = most_simplified_geometry.indices.len() / 3;

            lod_meshes.insert(
                LODLevel::LOD2,
                LODMeshData {
                    geometry: most_simplified_geometry,
                    mesh_handle: most_simplified_handle,
                    triangle_count: most_simplified_triangle_count,
                },
            );
            println!(
                "LOD2 最简化完成，生成 {} 个三角形",
                most_simplified_triangle_count
            );
        }

        // 初始使用最高精度（LOD0）
        let initial_lod = LODLevel::LOD0;

        Ok(LODManager {
            lod_meshes,
            current_lod: initial_lod,
            model_center,
            model_size,
            needs_update: false,
        })
    }

    /// 根据距离选择合适的LOD级别
    pub fn select_lod_by_distance(&self, distance: f32) -> LODLevel {
        // 根据模型大小调整距离阈值，对小模型使用更小的因子
        let size_factor = if self.model_size < 5.0 {
            // 小模型（如bunny）使用更小的距离因子
            (self.model_size / 5.0).max(0.3)
        } else {
            // 大模型保持原有逻辑
            (self.model_size / 10.0).max(1.0)
        };

        for level in LODLevel::all_levels() {
            if self.lod_meshes.contains_key(&level) {
                let threshold = level.distance_threshold() * size_factor;
                if distance <= threshold {
                    return level;
                }
            }
        }

        // 默认返回最低精度
        LODLevel::LOD2
    }

    /// 更新当前LOD级别
    pub fn update_lod(&mut self, camera_distance: f32) -> bool {
        let new_lod = self.select_lod_by_distance(camera_distance);
        if new_lod != self.current_lod {
            self.current_lod = new_lod;
            self.needs_update = true;
            
            // 计算实际使用的距离阈值用于调试
            let size_factor = if self.model_size < 5.0 {
                (self.model_size / 5.0).max(0.3)
            } else {
                (self.model_size / 10.0).max(1.0)
            };
            
            println!(
                "LOD切换到 {:?}，距离: {:.2}，模型大小: {:.2}，大小因子: {:.2}，LOD0阈值: {:.2}，LOD1阈值: {:.2}",
                new_lod, 
                camera_distance, 
                self.model_size,
                size_factor,
                LODLevel::LOD0.distance_threshold() * size_factor,
                LODLevel::LOD1.distance_threshold() * size_factor
            );
            true
        } else {
            false
        }
    }

    /// 获取当前LOD的网格句柄
    pub fn current_mesh_handle(&self) -> Option<&Handle<Mesh>> {
        self.lod_meshes
            .get(&self.current_lod)
            .map(|data| &data.mesh_handle)
    }

    /// 获取当前LOD的几何数据
    pub fn current_geometry(&self) -> Option<&GeometryData> {
        self.lod_meshes
            .get(&self.current_lod)
            .map(|data| &data.geometry)
    }
}

/// LOD系统插件
pub struct LODPlugin;

impl Plugin for LODPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                setup_lod_on_model_load,
                update_lod_based_on_camera_distance,
                update_lod_color_mapping,
            )
                .chain(),
        );
    }
}

/// 监听模型加载事件，自动设置LOD
fn setup_lod_on_model_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    current_model: Res<CurrentModelData>,
    model_entities: Query<Entity, (With<UserModelMesh>, Without<LODManager>)>,
) {
    // 检查是否有新加载的模型需要设置LOD
    if current_model.is_changed() {
        if let Some(ref geometry) = current_model.geometry {
            // 为所有用户模型实体添加LOD管理器
            for entity in model_entities.iter() {
                match LODManager::new(geometry.clone(), &mut meshes) {
                    Ok(lod_manager) => {
                        commands.entity(entity).insert(lod_manager);
                        println!("为模型实体 {:?} 添加LOD管理器", entity);
                    }
                    Err(e) => {
                        println!("创建LOD管理器失败: {:?}", e);
                    }
                }
            }
        }
    }
}

/// 根据相机距离更新LOD
fn update_lod_based_on_camera_distance(
    camera_query: Query<&Transform, (With<Camera3d>, Without<LODManager>)>,
    mut lod_entities: Query<(&mut LODManager, &mut Mesh3d), With<UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    for (mut lod_manager, mut mesh3d) in lod_entities.iter_mut() {
        // 计算相机到模型中心的距离
        let distance = camera_transform
            .translation
            .distance(lod_manager.model_center);

        // 更新LOD级别
        if lod_manager.update_lod(distance) {
            // 如果LOD级别发生变化，更新网格
            if let Some(new_mesh_handle) = lod_manager.current_mesh_handle() {
                let mesh_handle_clone = new_mesh_handle.clone();
                *mesh3d = Mesh3d(mesh_handle_clone.clone());
                lod_manager.needs_update = false;

                // 应用当前的颜色映射到新的LOD网格
                if let (Some(mesh), Some(current_geometry)) = (
                    meshes.get_mut(&mesh_handle_clone),
                    lod_manager.current_geometry(),
                ) {
                    if let Err(e) = crate::ui::color_bar::apply_custom_color_mapping(
                        current_geometry,
                        mesh,
                        &color_bar_config,
                    ) {
                        println!("Failed to apply color mapping to LOD mesh: {:?}", e);
                    }
                }
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 计算顶点数组的包围盒
fn calculate_bounding_box(vertices: &Vec<[f32; 3]>) -> (Vec3, f32) {
    if vertices.is_empty() {
        return (Vec3::ZERO, 1.0);
    }

    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

    for vertex in vertices {
        let v = Vec3::new(vertex[0], vertex[1], vertex[2]);
        min = min.min(v);
        max = max.max(v);
    }

    let center = (min + max) * 0.5;
    let size = (max - min).length();

    (center, size)
}

/// 简化网格（保留指定比例的三角形）
fn simplify_mesh(geometry: &GeometryData, ratio: f32) -> Result<GeometryData, VtkError> {
    let ratio = ratio.clamp(0.1, 1.0);
    let original_triangle_count = geometry.indices.len() / 3;
    let target_triangle_count = ((original_triangle_count as f32) * ratio) as usize;

    println!(
        "简化网格：从 {} 个三角形简化到 {} 个三角形",
        original_triangle_count, target_triangle_count
    );

    // 使用Quadric Error Metrics算法进行简化
    simplify_mesh_qem(geometry, ratio)
}

/// 基于Quadric Error Metrics的网格简化算法
fn simplify_mesh_qem(geometry: &GeometryData, ratio: f32) -> Result<GeometryData, VtkError> {
    // 如果简化比例过高，使用备用算法
    if ratio < 0.2 {
        println!("QEM简化比例过低 ({})，使用顶点聚类算法", ratio);
        return simplify_mesh_vertex_clustering(geometry, ratio);
    }

    let target_triangle_count = ((geometry.indices.len() / 3) as f32 * ratio) as usize;

    // 构建半边数据结构
    let mut mesh = QEMMesh::from_geometry(geometry);

    // 计算每个顶点的quadric
    mesh.compute_vertex_quadrics();

    // 计算所有边的collapse成本
    mesh.compute_edge_costs();

    // 执行边坍缩，直到达到目标三角形数
    let current_triangle_count = mesh.triangle_count();
    let mut max_collapses = current_triangle_count.saturating_sub(target_triangle_count);

    // 限制最大坍缩次数，保证不会过度简化
    max_collapses = max_collapses.min(current_triangle_count / 2);

    println!("QEM简化：计划坍缩最多 {} 条边", max_collapses);

    let mut collapsed_count = 0;
    let mut consecutive_failures = 0; // 连续失败次数

    for _i in 0..max_collapses {
        let triangle_count_before = mesh.triangle_count();
        if triangle_count_before <= target_triangle_count {
            break;
        }

        // 保留最少数量的三角形，避免过度简化
        if triangle_count_before < 30 {
            break;
        }

        if !mesh.collapse_cheapest_edge() {
            consecutive_failures += 1;
            if consecutive_failures > 5 {
                break; // 连续失败多次，停止简化
            }
        } else {
            consecutive_failures = 0; // 重置失败计数
            collapsed_count += 1;
        }
    }

    println!(
        "QEM简化完成：坍缩了 {} 条边，实际生成 {} 个三角形",
        collapsed_count,
        mesh.triangle_count()
    );

    // 转换回GeometryData
    mesh.to_geometry_data()
}

/// QEM网格数据结构
struct QEMMesh {
    vertices: Vec<QEMVertex>,
    edges: Vec<QEMEdge>,
    triangles: Vec<QEMTriangle>,
    #[allow(dead_code)]
    vertex_mapping: HashMap<usize, usize>, // 原始顶点索引 -> QEM顶点索引
    // 保存原始几何数据的Cell属性
    original_cell_attributes: Option<HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>>,
}

/// QEM顶点
#[derive(Clone)]
struct QEMVertex {
    position: [f32; 3],
    quadric: QuadricMatrix,
    edges: Vec<usize>, // 连接的边索引
    is_deleted: bool,
    original_attributes: Option<OriginalVertexAttribs>, // 保存原始属性
}

/// QEM边
#[derive(Clone)]
struct QEMEdge {
    v0: usize,
    v1: usize,
    cost: f32,
    optimal_position: [f32; 3],
    triangles: Vec<usize>, // 包含此边的三角形
    is_deleted: bool,
}

/// QEM三角形
#[derive(Clone)]
struct QEMTriangle {
    vertices: [usize; 3],
    plane: [f32; 4], // 平面方程 ax + by + cz + d = 0
    is_deleted: bool,
}

/// Quadric矩阵 (4x4对称矩阵)
#[derive(Clone)]
struct QuadricMatrix {
    // 存储上三角矩阵的10个元素
    // Q = [q11 q12 q13 q14]
    //     [q12 q22 q23 q24]
    //     [q13 q23 q33 q34]
    //     [q14 q24 q34 q44]
    q: [f64; 10], // q11,q12,q13,q14,q22,q23,q24,q33,q34,q44
}

/// 原始顶点属性
#[derive(Clone)]
struct OriginalVertexAttribs {
    scalar_values: HashMap<String, f32>,
    vector_values: HashMap<String, [f32; 3]>,
    color_values: HashMap<String, Vec<f32>>,
}

impl QuadricMatrix {
    fn new() -> Self {
        Self { q: [0.0; 10] }
    }

    /// 从三角形平面创建quadric
    fn from_plane(plane: [f32; 4]) -> Self {
        let [a, b, c, d] = plane.map(|x| x as f64);
        Self {
            q: [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ],
        }
    }

    /// 添加另一个quadric
    fn add(&mut self, other: &QuadricMatrix) {
        for i in 0..10 {
            self.q[i] += other.q[i];
        }
    }

    /// 计算顶点的error
    fn error(&self, v: [f32; 3]) -> f64 {
        let [x, y, z] = v.map(|x| x as f64);
        let w = 1.0;

        // v^T * Q * v
        self.q[0] * x * x
            + 2.0 * self.q[1] * x * y
            + 2.0 * self.q[2] * x * z
            + 2.0 * self.q[3] * x * w
            + self.q[4] * y * y
            + 2.0 * self.q[5] * y * z
            + 2.0 * self.q[6] * y * w
            + self.q[7] * z * z
            + 2.0 * self.q[8] * z * w
            + self.q[9] * w * w
    }

    /// 找到最优位置（通过解线性方程组）
    fn optimal_position(&self) -> Option<[f32; 3]> {
        // 构造3x3矩阵 A 和向量 b，求解 Ax = -b
        let a11 = self.q[0]; // q11
        let a12 = self.q[1]; // q12
        let a13 = self.q[2]; // q13
        let a22 = self.q[4]; // q22
        let a23 = self.q[5]; // q23
        let a33 = self.q[7]; // q33

        let b1 = self.q[3]; // q14
        let b2 = self.q[6]; // q24
        let b3 = self.q[8]; // q34

        // 计算行列式
        let det = a11 * (a22 * a33 - a23 * a23) - a12 * (a12 * a33 - a23 * a13)
            + a13 * (a12 * a23 - a22 * a13);

        if det.abs() < 1e-12 {
            return None; // 矩阵奇异
        }

        // 使用Cramer法则求解
        let x = ((-b1) * (a22 * a33 - a23 * a23) - a12 * ((-b2) * a33 - a23 * (-b3))
            + a13 * ((-b2) * a23 - a22 * (-b3)))
            / det;
        let y = (a11 * ((-b2) * a33 - a23 * (-b3)) - (-b1) * (a12 * a33 - a23 * a13)
            + a13 * (a12 * (-b3) - (-b2) * a13))
            / det;
        let z = (a11 * (a22 * (-b3) - (-b2) * a23) - a12 * (a12 * (-b3) - (-b2) * a13)
            + (-b1) * (a12 * a23 - a22 * a13))
            / det;

        Some([x as f32, y as f32, z as f32])
    }
}

impl QEMMesh {
    fn from_geometry(geometry: &GeometryData) -> Self {
        let mut vertices = Vec::new();
        let mut vertex_mapping = HashMap::new();

        // 提取原始属性
        let original_attrs = Self::extract_original_attributes(geometry);

        // 创建顶点
        for (i, &pos) in geometry.vertices.iter().enumerate() {
            let attribs = original_attrs.get(&i).cloned();
            vertices.push(QEMVertex {
                position: pos,
                quadric: QuadricMatrix::new(),
                edges: Vec::new(),
                is_deleted: false,
                original_attributes: attribs,
            });
            vertex_mapping.insert(i, i);
        }

        // 创建三角形
        let mut triangles = Vec::new();
        for chunk in geometry.indices.chunks(3) {
            if chunk.len() != 3 {
                continue;
            }

            let v0 = chunk[0] as usize;
            let v1 = chunk[1] as usize;
            let v2 = chunk[2] as usize;

            // 计算三角形法向量和平面方程
            let p0 = Vec3::from(vertices[v0].position);
            let p1 = Vec3::from(vertices[v1].position);
            let p2 = Vec3::from(vertices[v2].position);

            let normal = (p1 - p0).cross(p2 - p0).normalize();
            let d = -normal.dot(p0);

            triangles.push(QEMTriangle {
                vertices: [v0, v1, v2],
                plane: [normal.x, normal.y, normal.z, d],
                is_deleted: false,
            });
        }

        // 构建边
        let edges = Self::build_edges(&triangles);

        // 更新顶点的边连接
        Self::update_vertex_edges(&mut vertices, &edges);

        // 保存原始Cell属性
        let original_cell_attributes = if let Some(ref attrs) = geometry.attributes {
            let mut cell_attrs = HashMap::new();
            for ((name, location), attr_type) in attrs {
                if let crate::mesh::vtk::AttributeLocation::Cell = location {
                    cell_attrs.insert((name.clone(), location.clone()), attr_type.clone());
                }
            }
            if cell_attrs.is_empty() {
                None
            } else {
                Some(cell_attrs)
            }
        } else {
            None
        };

        QEMMesh {
            vertices,
            edges,
            triangles,
            vertex_mapping,
            original_cell_attributes,
        }
    }

    fn extract_original_attributes(
        geometry: &GeometryData,
    ) -> HashMap<usize, OriginalVertexAttribs> {
        let mut attrs_map = HashMap::new();

        if let Some(ref attrs) = geometry.attributes {
            for i in 0..geometry.vertices.len() {
                let mut vertex_attrs = OriginalVertexAttribs {
                    scalar_values: HashMap::new(),
                    vector_values: HashMap::new(),
                    color_values: HashMap::new(),
                };

                for ((name, location), attr_type) in attrs {
                    if let crate::mesh::vtk::AttributeLocation::Point = location {
                        match attr_type {
                            crate::mesh::vtk::AttributeType::Scalar { data, .. } => {
                                if i < data.len() {
                                    vertex_attrs.scalar_values.insert(name.clone(), data[i]);
                                }
                            }
                            crate::mesh::vtk::AttributeType::Vector(vectors) => {
                                if i < vectors.len() {
                                    vertex_attrs.vector_values.insert(name.clone(), vectors[i]);
                                }
                            }
                            crate::mesh::vtk::AttributeType::ColorScalar { data, .. } => {
                                if i < data.len() {
                                    vertex_attrs
                                        .color_values
                                        .insert(name.clone(), data[i].clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                attrs_map.insert(i, vertex_attrs);
            }
        }

        attrs_map
    }

    fn build_edges(triangles: &[QEMTriangle]) -> Vec<QEMEdge> {
        let mut edge_map: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

        // 收集所有边
        for (tri_idx, triangle) in triangles.iter().enumerate() {
            let [v0, v1, v2] = triangle.vertices;
            let edges = [
                (v0.min(v1), v0.max(v1)),
                (v1.min(v2), v1.max(v2)),
                (v2.min(v0), v2.max(v0)),
            ];

            for edge in edges {
                edge_map.entry(edge).or_insert_with(Vec::new).push(tri_idx);
            }
        }

        // 创建边对象
        edge_map
            .into_iter()
            .map(|((v0, v1), triangles)| QEMEdge {
                v0,
                v1,
                cost: f32::INFINITY,
                optimal_position: [0.0; 3],
                triangles,
                is_deleted: false,
            })
            .collect()
    }

    fn update_vertex_edges(vertices: &mut [QEMVertex], edges: &[QEMEdge]) {
        for vertex in vertices.iter_mut() {
            vertex.edges.clear();
        }

        for (edge_idx, edge) in edges.iter().enumerate() {
            if !edge.is_deleted {
                vertices[edge.v0].edges.push(edge_idx);
                vertices[edge.v1].edges.push(edge_idx);
            }
        }
    }

    fn compute_vertex_quadrics(&mut self) {
        // 重置所有quadric
        for vertex in &mut self.vertices {
            vertex.quadric = QuadricMatrix::new();
        }

        // 累加每个三角形的quadric到其顶点
        for triangle in &self.triangles {
            if triangle.is_deleted {
                continue;
            }

            let quadric = QuadricMatrix::from_plane(triangle.plane);
            for &vertex_idx in &triangle.vertices {
                self.vertices[vertex_idx].quadric.add(&quadric);
            }
        }
    }

    fn compute_edge_costs(&mut self) {
        for edge in &mut self.edges {
            if edge.is_deleted {
                continue;
            }

            let v0_quadric = &self.vertices[edge.v0].quadric;
            let v1_quadric = &self.vertices[edge.v1].quadric;

            // 合并quadric
            let mut combined_quadric = v0_quadric.clone();
            combined_quadric.add(v1_quadric);

            // 尝试找到最优位置
            if let Some(optimal_pos) = combined_quadric.optimal_position() {
                edge.optimal_position = optimal_pos;
                edge.cost = combined_quadric.error(optimal_pos) as f32;
            } else {
                // 如果无法求解最优位置，尝试端点中点
                let v0_pos = self.vertices[edge.v0].position;
                let v1_pos = self.vertices[edge.v1].position;
                let midpoint = [
                    (v0_pos[0] + v1_pos[0]) * 0.5,
                    (v0_pos[1] + v1_pos[1]) * 0.5,
                    (v0_pos[2] + v1_pos[2]) * 0.5,
                ];

                let cost_v0 = combined_quadric.error(v0_pos);
                let cost_v1 = combined_quadric.error(v1_pos);
                let cost_mid = combined_quadric.error(midpoint);

                // 选择成本最低的位置
                if cost_v0 <= cost_v1 && cost_v0 <= cost_mid {
                    edge.optimal_position = v0_pos;
                    edge.cost = cost_v0 as f32;
                } else if cost_v1 <= cost_mid {
                    edge.optimal_position = v1_pos;
                    edge.cost = cost_v1 as f32;
                } else {
                    edge.optimal_position = midpoint;
                    edge.cost = cost_mid as f32;
                }
            }
        }
    }

    fn collapse_cheapest_edge(&mut self) -> bool {
        // 找到成本最低的边
        let mut best_edge_idx = None;
        let mut best_cost = f32::INFINITY;

        for (edge_idx, edge) in self.edges.iter().enumerate() {
            if !edge.is_deleted && edge.cost < best_cost {
                best_cost = edge.cost;
                best_edge_idx = Some(edge_idx);
            }
        }

        let Some(edge_idx) = best_edge_idx else {
            return false; // 没有可用的边
        };

        // 执行边坍缩
        self.collapse_edge(edge_idx)
    }

    fn collapse_edge(&mut self, edge_idx: usize) -> bool {
        let edge = self.edges[edge_idx].clone();
        if edge.is_deleted {
            return false;
        }

        let v0_idx = edge.v0;
        let v1_idx = edge.v1;

        // 将v1坍缩到v0，并更新v0的位置
        self.vertices[v0_idx].position = edge.optimal_position;

        // 合并属性（简单平均）
        if let (Some(ref attrs0), Some(ref attrs1)) = (
            &self.vertices[v0_idx].original_attributes,
            &self.vertices[v1_idx].original_attributes,
        ) {
            let mut merged_attrs = attrs0.clone();

            // 合并标量属性
            for (name, &value1) in &attrs1.scalar_values {
                if let Some(&value0) = merged_attrs.scalar_values.get(name) {
                    merged_attrs
                        .scalar_values
                        .insert(name.clone(), (value0 + value1) * 0.5);
                }
            }

            // 合并向量属性
            for (name, &vector1) in &attrs1.vector_values {
                if let Some(&vector0) = merged_attrs.vector_values.get(name) {
                    let merged = [
                        (vector0[0] + vector1[0]) * 0.5,
                        (vector0[1] + vector1[1]) * 0.5,
                        (vector0[2] + vector1[2]) * 0.5,
                    ];
                    merged_attrs.vector_values.insert(name.clone(), merged);
                }
            }

            self.vertices[v0_idx].original_attributes = Some(merged_attrs);
        }

        // 合并quadric
        let v1_quadric = self.vertices[v1_idx].quadric.clone();
        self.vertices[v0_idx].quadric.add(&v1_quadric);

        // 首先更新所有引用v1的三角形，让它们引用v0
        for triangle in &mut self.triangles {
            if triangle.is_deleted {
                continue;
            }
            for vertex_ref in &mut triangle.vertices {
                if *vertex_ref == v1_idx {
                    *vertex_ref = v0_idx;
                }
            }
        }

        // 删除包含此边的三角形（这些三角形现在会有重复的顶点）
        for &tri_idx in &edge.triangles {
            if !self.triangles[tri_idx].is_deleted {
                self.triangles[tri_idx].is_deleted = true;
            }
        }

        // 删除退化的三角形（有重复顶点的三角形）
        for triangle in &mut self.triangles {
            if triangle.is_deleted {
                continue;
            }
            let [v0, v1, v2] = triangle.vertices;
            if v0 == v1 || v1 == v2 || v2 == v0 {
                triangle.is_deleted = true;
            }
        }

        // 将v1的所有连接转移到v0
        let v1_edges: Vec<usize> = self.vertices[v1_idx].edges.clone();
        for &other_edge_idx in &v1_edges {
            if other_edge_idx == edge_idx {
                continue;
            }

            let other_edge = &mut self.edges[other_edge_idx];
            if other_edge.is_deleted {
                continue;
            }

            if other_edge.v0 == v1_idx {
                other_edge.v0 = v0_idx;
            } else if other_edge.v1 == v1_idx {
                other_edge.v1 = v0_idx;
            }

            // 删除退化的边（两端是同一个顶点）
            if other_edge.v0 == other_edge.v1 {
                other_edge.is_deleted = true;
            }
        }

        // 删除v1和当前边
        self.vertices[v1_idx].is_deleted = true;
        self.edges[edge_idx].is_deleted = true;

        // 重新计算受影响顶点的quadric和边成本
        Self::update_vertex_edges(&mut self.vertices, &self.edges);

        // 重新计算v0附近的边成本
        let v0_edges: Vec<usize> = self.vertices[v0_idx].edges.clone();
        for &edge_idx in &v0_edges {
            if !self.edges[edge_idx].is_deleted {
                self.compute_single_edge_cost(edge_idx);
            }
        }

        true
    }

    fn compute_single_edge_cost(&mut self, edge_idx: usize) {
        let edge = &mut self.edges[edge_idx];
        if edge.is_deleted {
            return;
        }

        let v0_quadric = &self.vertices[edge.v0].quadric;
        let v1_quadric = &self.vertices[edge.v1].quadric;

        let mut combined_quadric = v0_quadric.clone();
        combined_quadric.add(v1_quadric);

        if let Some(optimal_pos) = combined_quadric.optimal_position() {
            edge.optimal_position = optimal_pos;
            edge.cost = combined_quadric.error(optimal_pos) as f32;
        } else {
            let v0_pos = self.vertices[edge.v0].position;
            let v1_pos = self.vertices[edge.v1].position;
            let midpoint = [
                (v0_pos[0] + v1_pos[0]) * 0.5,
                (v0_pos[1] + v1_pos[1]) * 0.5,
                (v0_pos[2] + v1_pos[2]) * 0.5,
            ];

            let cost_v0 = combined_quadric.error(v0_pos);
            let cost_v1 = combined_quadric.error(v1_pos);
            let cost_mid = combined_quadric.error(midpoint);

            if cost_v0 <= cost_v1 && cost_v0 <= cost_mid {
                edge.optimal_position = v0_pos;
                edge.cost = cost_v0 as f32;
            } else if cost_v1 <= cost_mid {
                edge.optimal_position = v1_pos;
                edge.cost = cost_v1 as f32;
            } else {
                edge.optimal_position = midpoint;
                edge.cost = cost_mid as f32;
            }
        }
    }

    fn triangle_count(&self) -> usize {
        self.triangles.iter().filter(|t| !t.is_deleted).count()
    }

    fn to_geometry_data(&self) -> Result<GeometryData, VtkError> {
        // 收集有效顶点
        let mut vertex_map = HashMap::new();
        let mut new_vertices = Vec::new();

        for (old_idx, vertex) in self.vertices.iter().enumerate() {
            if !vertex.is_deleted {
                let new_idx = new_vertices.len();
                new_vertices.push(vertex.position);
                vertex_map.insert(old_idx, new_idx as u32);
            }
        }

        // 收集有效三角形
        let mut new_indices = Vec::new();
        let mut triangle_to_cell_mapping = Vec::new();
        let mut cell_index = 0;
        
        for triangle in &self.triangles {
            if triangle.is_deleted {
                continue;
            }

            let [v0, v1, v2] = triangle.vertices;
            if let (Some(&new_v0), Some(&new_v1), Some(&new_v2)) = (
                vertex_map.get(&v0),
                vertex_map.get(&v1),
                vertex_map.get(&v2),
            ) {
                new_indices.extend_from_slice(&[new_v0, new_v1, new_v2]);
                // 为每个新三角形分配一个cell索引
                triangle_to_cell_mapping.push(cell_index);
                cell_index += 1;
            }
        }

        // 重建属性
        let new_attributes = self.rebuild_attributes(&vertex_map, new_vertices.len())?;

        let mut geometry = GeometryData::new(new_vertices, new_indices, new_attributes);
        
        // 添加三角形到cell的映射
        geometry = geometry.add_triangle_to_cell_mapping(triangle_to_cell_mapping);

        Ok(geometry)
    }

    fn rebuild_attributes(
        &self,
        vertex_map: &HashMap<usize, u32>,
        new_vertex_count: usize,
    ) -> Result<
        HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>,
        VtkError,
    > {
        let mut new_attrs = HashMap::new();

        // 收集所有属性名称
        let mut scalar_names = std::collections::HashSet::new();
        let mut vector_names = std::collections::HashSet::new();
        let mut color_names = std::collections::HashSet::new();

        for vertex in &self.vertices {
            if vertex.is_deleted {
                continue;
            }
            if let Some(ref attrs) = vertex.original_attributes {
                scalar_names.extend(attrs.scalar_values.keys().cloned());
                vector_names.extend(attrs.vector_values.keys().cloned());
                color_names.extend(attrs.color_values.keys().cloned());
            }
        }

        // 重建标量属性
        for name in scalar_names {
            let mut data = vec![0.0; new_vertex_count];
            for (old_idx, vertex) in self.vertices.iter().enumerate() {
                if vertex.is_deleted {
                    continue;
                }
                if let Some(new_idx) = vertex_map.get(&old_idx) {
                    if let Some(ref attrs) = vertex.original_attributes {
                        if let Some(&value) = attrs.scalar_values.get(&name) {
                            data[*new_idx as usize] = value;
                        }
                    }
                }
            }

            let attr = crate::mesh::vtk::AttributeType::Scalar {
                num_comp: 1,
                table_name: "default".to_string(),
                data,
                lookup_table: None,
            };
            new_attrs.insert((name, crate::mesh::vtk::AttributeLocation::Point), attr);
        }

        // 重建向量属性
        for name in vector_names {
            let mut data = vec![[0.0; 3]; new_vertex_count];
            for (old_idx, vertex) in self.vertices.iter().enumerate() {
                if vertex.is_deleted {
                    continue;
                }
                if let Some(new_idx) = vertex_map.get(&old_idx) {
                    if let Some(ref attrs) = vertex.original_attributes {
                        if let Some(&value) = attrs.vector_values.get(&name) {
                            data[*new_idx as usize] = value;
                        }
                    }
                }
            }

            let attr = crate::mesh::vtk::AttributeType::Vector(data);
            new_attrs.insert((name, crate::mesh::vtk::AttributeLocation::Point), attr);
        }

        // 重建Cell属性（处理原始的Cell属性）
        if let Some(ref original_cell_attrs) = self.original_cell_attributes {
            for ((name, location), attr_type) in original_cell_attrs {
                let new_triangle_count = self.triangles.iter().filter(|t| !t.is_deleted).count();
                
                match attr_type {
                    crate::mesh::vtk::AttributeType::Scalar { table_name, .. } => {
                        // 为简化后的每个Cell（三角形）分配相同的标量值
                        // 这里使用第一个有效Cell的值作为所有简化Cell的值
                        let mut cell_data = vec![1.0; new_triangle_count]; // 默认值
                        
                        if let crate::mesh::vtk::AttributeType::Scalar { data: original_data, .. } = attr_type {
                            if !original_data.is_empty() {
                                let default_value = original_data[0]; // 使用第一个值
                                cell_data.fill(default_value);
                                println!("重建Cell属性 '{}': {} 个Cell，值={}", name, new_triangle_count, default_value);
                            }
                        }

                        let new_attr = crate::mesh::vtk::AttributeType::Scalar {
                            num_comp: 1,
                            table_name: table_name.clone(),
                            data: cell_data,
                            lookup_table: None,
                        };
                        new_attrs.insert((name.clone(), location.clone()), new_attr);
                    }
                    _ => {
                        // 可以扩展支持其他Cell属性类型
                    }
                }
            }
        }

        Ok(new_attrs)
    }
}

/// 基于顶点聚类的网格简化算法
fn simplify_mesh_vertex_clustering(
    geometry: &GeometryData,
    ratio: f32,
) -> Result<GeometryData, VtkError> {
    let target_triangle_count = ((geometry.indices.len() / 3) as f32 * ratio) as usize;

    // 计算包围盒
    let (center, size) = calculate_bounding_box(&geometry.vertices);

    // 创建网格分辨率（根据目标简化比例调整，使用更保守的设置避免破洞）
    let grid_resolution = (20.0 * ratio.sqrt()).max(8.0) as usize;
    let cell_size = size / grid_resolution as f32;

    // 网格聚类：将相近的顶点合并
    let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();

    for (vertex_idx, vertex) in geometry.vertices.iter().enumerate() {
        let pos = Vec3::from(*vertex);
        let grid_pos = (
            ((pos.x - center.x + size * 0.5) / cell_size) as i32,
            ((pos.y - center.y + size * 0.5) / cell_size) as i32,
            ((pos.z - center.z + size * 0.5) / cell_size) as i32,
        );

        grid.entry(grid_pos)
            .or_insert_with(Vec::new)
            .push(vertex_idx);
    }

    // 为每个网格单元选择代表顶点
    let mut vertex_mapping: HashMap<usize, usize> = HashMap::new();
    let mut new_vertices = Vec::new();
    let mut representative_vertices: HashMap<(i32, i32, i32), usize> = HashMap::new();

    for (grid_pos, vertices_in_cell) in grid.iter() {
        // 选择网格单元中心最近的顶点作为代表
        let cell_center = Vec3::new(
            center.x + (grid_pos.0 as f32 + 0.5) * cell_size - size * 0.5,
            center.y + (grid_pos.1 as f32 + 0.5) * cell_size - size * 0.5,
            center.z + (grid_pos.2 as f32 + 0.5) * cell_size - size * 0.5,
        );

        let representative = vertices_in_cell
            .iter()
            .min_by(|&&a, &&b| {
                let dist_a = Vec3::from(geometry.vertices[a]).distance(cell_center);
                let dist_b = Vec3::from(geometry.vertices[b]).distance(cell_center);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .copied()
            .unwrap();

        let new_vertex_idx = new_vertices.len();
        new_vertices.push(geometry.vertices[representative]);
        representative_vertices.insert(*grid_pos, new_vertex_idx);

        // 将该网格单元中的所有顶点映射到代表顶点
        for &vertex_idx in vertices_in_cell {
            vertex_mapping.insert(vertex_idx, new_vertex_idx);
        }
    }

    // 重建三角形，去除重复和退化的三角形
    let mut new_indices = Vec::new();
    let mut triangle_set = std::collections::HashSet::new();

    for chunk in geometry.indices.chunks(3) {
        if chunk.len() != 3 {
            continue;
        }

        let v0 = vertex_mapping[&(chunk[0] as usize)];
        let v1 = vertex_mapping[&(chunk[1] as usize)];
        let v2 = vertex_mapping[&(chunk[2] as usize)];

        // 跳过退化的三角形（有重复顶点）
        if v0 == v1 || v1 == v2 || v2 == v0 {
            continue;
        }

        // 检查三角形质量，避免过小或退化的三角形
        let p0 = Vec3::from(new_vertices[v0]);
        let p1 = Vec3::from(new_vertices[v1]);
        let p2 = Vec3::from(new_vertices[v2]);

        // 计算三角形面积
        let area = 0.5 * (p1 - p0).cross(p2 - p0).length();
        let min_area = (size * size) * 1e-6; // 相对于模型大小的最小面积阈值

        if area < min_area {
            continue; // 跳过过小的三角形
        }

        // 创建规范化的三角形（按顶点索引排序以避免重复）
        let mut triangle = [v0, v1, v2];
        triangle.sort();

        if triangle_set.insert(triangle) {
            new_indices.extend_from_slice(&[v0 as u32, v1 as u32, v2 as u32]);
        }

        // 如果已经达到目标三角形数量，停止添加
        if new_indices.len() / 3 >= target_triangle_count {
            break;
        }
    }

    println!("简化完成：实际生成 {} 个三角形", new_indices.len() / 3);

    // 简化属性数据
    let new_attributes = if let Some(ref attrs) = geometry.attributes {
        simplify_attributes_clustered(attrs, &vertex_mapping, new_vertices.len())?
    } else {
        HashMap::new()
    };

    Ok(GeometryData::new(new_vertices, new_indices, new_attributes))
}

/// 基于聚类的属性简化
fn simplify_attributes_clustered(
    original_attrs: &HashMap<
        (String, crate::mesh::vtk::AttributeLocation),
        crate::mesh::vtk::AttributeType,
    >,
    vertex_mapping: &HashMap<usize, usize>,
    new_vertex_count: usize,
) -> Result<
    HashMap<(String, crate::mesh::vtk::AttributeLocation), crate::mesh::vtk::AttributeType>,
    VtkError,
> {
    let mut new_attrs = HashMap::new();

    for ((name, location), attr_type) in original_attrs.iter() {
        match location {
            crate::mesh::vtk::AttributeLocation::Point => {
                let new_attr = simplify_point_attribute_clustered(
                    attr_type,
                    vertex_mapping,
                    new_vertex_count,
                )?;
                new_attrs.insert((name.clone(), location.clone()), new_attr);
            }
            crate::mesh::vtk::AttributeLocation::Cell => {
                println!("跳过单元格属性 '{}' 的简化", name);
            }
        }
    }

    Ok(new_attrs)
}

/// 基于聚类的点属性简化
fn simplify_point_attribute_clustered(
    attr_type: &crate::mesh::vtk::AttributeType,
    vertex_mapping: &HashMap<usize, usize>,
    new_vertex_count: usize,
) -> Result<crate::mesh::vtk::AttributeType, VtkError> {
    use crate::mesh::vtk::AttributeType;

    match attr_type {
        AttributeType::Scalar {
            data,
            num_comp,
            table_name,
            lookup_table,
        } => {
            let mut new_values = vec![0.0; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            // 累积映射到同一新顶点的所有原始顶点的属性值
            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < data.len() && new_idx < new_values.len() {
                    new_values[new_idx] += data[*old_idx];
                    value_counts[new_idx] += 1;
                }
            }

            // 计算平均值
            for (value, count) in new_values.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    *value /= *count as f32;
                }
            }

            Ok(AttributeType::Scalar {
                num_comp: *num_comp,
                table_name: table_name.clone(),
                data: new_values,
                lookup_table: lookup_table.clone(),
            })
        }
        AttributeType::ColorScalar { nvalues, data } => {
            let mut new_data = vec![vec![0.0; *nvalues as usize]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < data.len() && new_idx < new_data.len() {
                    for (i, &value) in data[*old_idx].iter().enumerate() {
                        if i < new_data[new_idx].len() {
                            new_data[new_idx][i] += value;
                        }
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // 计算平均值
            for (color_data, count) in new_data.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for value in color_data.iter_mut() {
                        *value /= *count as f32;
                    }
                }
            }

            Ok(AttributeType::ColorScalar {
                nvalues: *nvalues,
                data: new_data,
            })
        }
        AttributeType::Vector(vectors) => {
            let mut new_vectors = vec![[0.0; 3]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < vectors.len() && new_idx < new_vectors.len() {
                    for i in 0..3 {
                        new_vectors[new_idx][i] += vectors[*old_idx][i];
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // 计算平均值并规范化向量
            for (vector, count) in new_vectors.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for component in vector.iter_mut() {
                        *component /= *count as f32;
                    }
                    // 可选：规范化向量长度
                    let length =
                        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2])
                            .sqrt();
                    if length > 0.0 {
                        for component in vector.iter_mut() {
                            *component /= length;
                        }
                    }
                }
            }

            Ok(AttributeType::Vector(new_vectors))
        }
        AttributeType::Tensor(tensors) => {
            let mut new_tensors = vec![[0.0; 9]; new_vertex_count];
            let mut value_counts = vec![0; new_vertex_count];

            for (old_idx, &new_idx) in vertex_mapping.iter() {
                if *old_idx < tensors.len() && new_idx < new_tensors.len() {
                    for i in 0..9 {
                        new_tensors[new_idx][i] += tensors[*old_idx][i];
                    }
                    value_counts[new_idx] += 1;
                }
            }

            // 计算平均值
            for (tensor, count) in new_tensors.iter_mut().zip(value_counts.iter()) {
                if *count > 0 {
                    for component in tensor.iter_mut() {
                        *component /= *count as f32;
                    }
                }
            }

            Ok(AttributeType::Tensor(new_tensors))
        }
    }
}

/// 当颜色映射配置改变时，更新所有LOD网格的颜色
fn update_lod_color_mapping(
    mut lod_entities: Query<&mut LODManager, With<UserModelMesh>>,
    color_bar_config: Res<crate::ui::ColorBarConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // 检查颜色配置是否有变化
    if !color_bar_config.has_changed {
        return;
    }

    println!("颜色映射配置已更改，更新所有LOD网格颜色");

    for lod_manager in lod_entities.iter_mut() {
        // 更新所有LOD级别的网格颜色
        for (lod_level, lod_data) in lod_manager.lod_meshes.iter() {
            if let Some(mesh) = meshes.get_mut(&lod_data.mesh_handle) {
                if let Err(e) = crate::ui::color_bar::apply_custom_color_mapping(
                    &lod_data.geometry,
                    mesh,
                    &color_bar_config,
                ) {
                    println!("无法为 {:?} 级别应用颜色映射: {:?}", lod_level, e);
                }
            }
        }
    }
}
