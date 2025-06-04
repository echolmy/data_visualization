use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct OpenFileEvent;

#[derive(Event)]
pub struct LoadModelEvent(pub PathBuf);

#[derive(Event)]
pub struct ToggleWireframeEvent;

#[derive(Event)]
pub struct SubdivideMeshEvent;

#[derive(Event)]
pub struct GenerateWaveEvent;

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
