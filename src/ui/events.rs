use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct OpenFileEvent;

#[derive(Event)]
pub struct LoadModelEvent(pub PathBuf);

#[derive(Event)]
pub struct ToggleWireframeEvent;

#[derive(Event)]
pub struct ConvertToHigherOrderEvent {
    pub order: u32, // mesh order, 2 for second order, 3 for third order, etc.
}

impl Default for ConvertToHigherOrderEvent {
    fn default() -> Self {
        Self { order: 2 } // default to second order mesh
    }
}
