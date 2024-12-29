use bevy::prelude::*;
use std::path::PathBuf;

#[derive(Event)]
pub struct OpenFileEvent;

#[derive(Event)]
pub struct LoadModelEvent(pub PathBuf);
