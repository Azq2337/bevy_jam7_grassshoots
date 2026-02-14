use crate::components::ShapeType;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Menu,
    Playing,
    Paused,
    Win,
    GameOver,
}

#[derive(Resource, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    #[default]
    MergeToWin,
    Survival,
}

#[derive(Resource)]
pub struct AssetLoadTimer(pub Timer);

#[derive(Resource)]
pub struct GunTimer(pub Timer);

#[derive(Resource)]
pub struct GameAssets {
    pub bullet_mesh: Handle<Mesh>,
    pub bullet_mat: Handle<StandardMaterial>,
    pub target_transparent_mat: Handle<StandardMaterial>,
    pub shape_meshes: HashMap<ShapeType, Handle<Mesh>>,
    pub level_materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Resource)]
pub struct ShotBlocker(pub Timer);

#[derive(Resource)]
pub struct GameStats {
    pub start_time: f32,
    pub max_population: f32,
}

#[derive(Resource, Default)]
pub struct HighScore(pub f32);
