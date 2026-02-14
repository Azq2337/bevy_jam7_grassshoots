// use crate::components::*;
use crate::resources::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct Bgm;

pub fn manage_bgm(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    state: Res<State<GameState>>,
    game_mode: Res<GameMode>,
    bgm_query: Query<(Entity, &AudioSink), With<Bgm>>,
) {
    if state.is_changed() {
        // Stop current BGM
        for (entity, sink) in bgm_query.iter() {
            sink.stop();
            commands.entity(entity).despawn();
        }

        let bgm_handle = match state.get() {
            GameState::Menu => Some(game_assets.bgm_menu.clone()),
            GameState::Playing => match *game_mode {
                GameMode::MergeToWin { .. } => Some(game_assets.bgm_mode1.clone()),
                GameMode::Survival { .. } => Some(game_assets.bgm_mode2.clone()),
            },
            GameState::Win => None, // Keep playing or silence? Or maybe Win sound covers it?
            GameState::GameOver => None,
            _ => None,
        };

        if let Some(bgm) = bgm_handle {
            commands.spawn((
                AudioPlayer::new(bgm),
                PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::Linear(0.4)),
                Bgm,
            ));
        }
    }
}
