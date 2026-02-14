mod components;
mod resources;
mod systems;

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::resources::*;
use crate::systems::{gameplay, menu, player, setup, ui};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Grass Shoots".into(),
                    ..default()
                }),
                ..default()
            }),
            PhysicsPlugins::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.35, 0.6, 0.9)))
        .insert_resource(AssetLoadTimer(Timer::from_seconds(1.5, TimerMode::Once)))
        .insert_resource(GunTimer(Timer::from_seconds(0.15, TimerMode::Repeating)))
        .insert_resource(ShotBlocker(Timer::from_seconds(0.0, TimerMode::Once)))
        .insert_resource(GameStats {
            start_time: 0.0,
            max_population: 100.0,
        })
        .insert_resource(HighScores::default())
        .insert_resource(GameMode::default())
        .insert_resource(menu::SelectedDifficulty(8))
        .init_state::<GameState>()
        .add_systems(Startup, setup::setup)
        // Loading State
        .add_systems(
            OnEnter(GameState::Loading),
            (
                ui::spawn_loading_screen,
                setup::cleanup_level,
                setup::spawn_menu_camera,
            ),
        )
        .add_systems(
            Update,
            ui::tick_loading.run_if(in_state(GameState::Loading)),
        )
        .add_systems(OnExit(GameState::Loading), ui::despawn_loading_screen)
        // Menu State
        .add_systems(
            OnEnter(GameState::Menu),
            (ui::release_cursor, menu::spawn_menu_screen),
        )
        .add_systems(
            Update,
            menu::handle_menu_interaction.run_if(in_state(GameState::Menu)),
        )
        .add_systems(
            OnExit(GameState::Menu),
            (
                menu::despawn_menu_screen,
                ui::capture_cursor,
                setup::spawn_level,
                setup::despawn_menu_camera,
            ),
        )
        // Audio
        .add_systems(Update, systems::audio::manage_bgm)
        // Playing State
        .add_systems(
            Update,
            (
                ui::handle_pause_input,
                ui::handle_pause_buttons.run_if(in_state(GameState::Paused)),
                ui::lock_cursor_on_click,
                player::player_look,
                ui::update_rotation_ui,
            ),
        )
        .add_systems(
            Update,
            (
                player::player_move,
                player::update_fov,
                player::handle_shooting,
                player::despawn_bullets,
                gameplay::bullet_hit_target,
                gameplay::handle_respawns,
                gameplay::handle_grabbing,
                gameplay::update_grabbed_object,
                gameplay::handle_merging,
                gameplay::update_gun,
                gameplay::handle_oob,
                gameplay::check_game_over,
                gameplay::update_game_stats,
                gameplay::tick_shot_blocker,
                gameplay::magnetic_pull,
            )
                .run_if(in_state(GameState::Playing)),
        )
        // Paused State
        .add_systems(
            OnEnter(GameState::Paused),
            (ui::release_cursor, ui::pause_time, ui::spawn_pause_menu),
        )
        .add_systems(
            OnExit(GameState::Paused),
            (ui::capture_cursor, ui::unpause_time, ui::despawn_pause_menu),
        )
        // Win State (Mode 1 Win)
        .add_systems(
            OnEnter(GameState::Win),
            (ui::release_cursor, ui::spawn_win_screen),
        )
        .add_systems(
            Update,
            ui::handle_win_input.run_if(in_state(GameState::Win)),
        )
        .add_systems(
            OnExit(GameState::Win),
            (ui::despawn_win_screen, ui::capture_cursor),
        )
        // GameOver State (Mode 2 Loss)
        .add_systems(
            OnEnter(GameState::GameOver),
            (ui::release_cursor, ui::spawn_win_screen),
        ) // Re-using win screen spawner which handles both
        .add_systems(
            Update,
            ui::handle_win_input.run_if(in_state(GameState::GameOver)),
        )
        .add_systems(
            OnExit(GameState::GameOver),
            (ui::despawn_win_screen, ui::capture_cursor),
        )
        .run();
}
