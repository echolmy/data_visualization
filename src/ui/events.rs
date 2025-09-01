use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct LoadModelEvent(pub PathBuf);

#[derive(Event)]
pub struct ToggleWireframeEvent;

#[derive(Event)]
pub struct SubdivideMeshEvent;

#[derive(Event)]
pub struct GenerateWaveEvent;

#[derive(Event)]
pub struct GenerateWaveShaderEvent;

#[derive(Event)]
pub struct ClearAllMeshesEvent;

#[derive(Event)]
pub struct GenerateLODEvent;

impl Default for SubdivideMeshEvent {
    fn default() -> Self {
        Self
    }
}

impl Default for GenerateWaveEvent {
    fn default() -> Self {
        Self
    }
}

impl Default for GenerateWaveShaderEvent {
    fn default() -> Self {
        Self
    }
}

impl Default for ClearAllMeshesEvent {
    fn default() -> Self {
        Self
    }
}

impl Default for GenerateLODEvent {
    fn default() -> Self {
        Self
    }
}
