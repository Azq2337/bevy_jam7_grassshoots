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

#[derive(Resource, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameMode {
    MergeToWin { target_level: u32 },
    Survival { target_level: u32 },
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::MergeToWin { target_level: 8 }
    }
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
    // Audio
    pub bgm_menu: Handle<AudioSource>,
    pub bgm_mode1: Handle<AudioSource>,
    pub bgm_mode2: Handle<AudioSource>,
    pub shoot: Handle<AudioSource>,
    pub merge: Handle<AudioSource>,
    pub win: Handle<AudioSource>,
    pub game_over: Handle<AudioSource>,
    pub click: Handle<AudioSource>,
    pub footstep: Handle<AudioSource>,
    pub spawn: Handle<AudioSource>,
    pub destroy: Handle<AudioSource>,
}

#[derive(Resource)]
pub struct ShotBlocker(pub Timer);

#[derive(Resource)]
pub struct GameStats {
    pub start_time: f32,
    pub max_population: f32,
}

#[derive(Resource, Default)]
pub struct HighScores {
    pub merge_to_win_best_time: f32, // Lower is better
    pub survival_max_time: f32,      // Higher is better
}
