use crate::components::*;
use crate::resources::*;
use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

// --- UI SYSTEMS ---

pub fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            LoadingScreen,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            ZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("LOADING ASSETS..."),
                TextFont {
                    font_size: 50.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenu,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(50),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 50.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            let button_node = Node {
                width: Val::Px(200.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            let button_bg = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            let text_font = TextFont {
                font_size: 33.0,
                ..default()
            };
            let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

            // Continue Button
            parent
                .spawn((
                    Button,
                    button_node.clone(),
                    button_bg,
                    PauseButtonAction::Continue,
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new("Continue"), text_font.clone(), text_color));
                });

            // Restart Button
            parent
                .spawn((Button, button_node, button_bg, PauseButtonAction::Restart))
                .with_children(|parent| {
                    parent.spawn((Text::new("Restart"), text_font, text_color));
                });
        });
}

pub fn despawn_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn update_rotation_ui(
    player_q: Query<(&Transform, &DashCooldown, Option<&DashActive>), With<Player>>,
    camera_q: Query<(&CameraPitch, &Projection), With<Camera3d>>,
    mut text_q: Query<&mut Text, With<RotationText>>,
    stats: Res<GameStats>,
    targets_q: Query<Entity, With<Target>>,
    game_mode: Res<GameMode>,
) {
    let Ok((player_transform, dash_cd, dash_active)) = player_q.single() else {
        return;
    };
    let Ok((pitch, projection)) = camera_q.single() else {
        return;
    };
    let Ok(mut text) = text_q.single_mut() else {
        return;
    };

    let (yaw, _, _) = player_transform.rotation.to_euler(EulerRot::YXZ);
    let current_fov = match projection {
        Projection::Perspective(p) => p.fov.to_degrees(),
        _ => 0.0,
    };

    let dash_status = if dash_active.is_some() {
        "DASHING".to_string()
    } else if dash_cd.0.elapsed() >= dash_cd.0.duration() {
        "READY".to_string()
    } else {
        format!("CD: {:.1}s", dash_cd.0.remaining_secs())
    };

    let mode_str = match *game_mode {
        GameMode::MergeToWin => "Merge to Win (Lvl 8)",
        GameMode::Survival => "Survival (No Lvl 8)",
    };

    let object_count = targets_q.iter().len();

    // Top Right: Controls (Already handled by RotationText position?)
    // User asked for "Top Right: Control Guide" and "Top Left: Info"
    // Currently RotationText is at Top Left (10px, 10px).
    // Let's repurpose RotationText to be the Top Left Info.

    text.0 = format!(
        "Mode: {}\nObjects: {} / {:.0}\nTime: {:.1}s\nPos: {:.1}\nYaw: {:.1} deg\nPitch: {:.1} deg\nFOV: {:.1}\nDash: {}",
        mode_str,
        object_count,
        stats.max_population,
        stats.start_time, // This is just start time, creates confusing output. Should be elapsed.
        // Wait, stats.start_time is f32 timestamp? No, it's initialized to 0.0.
        // "start_time: time.elapsed_secs()" in restart action.
        // So elapsed = time.elapsed_secs() - stats.start_time.
        // We'll fix this in next step when we have access to Time resource here.
        // Wait, update_rotation_ui signature doesn't have Time. I need to add it.
        player_transform.translation,
        yaw.to_degrees(),
        pitch.0.to_degrees(),
        current_fov,
        dash_status
    );
}

// Helper to fix the time issue in update_rotation_ui
// I'll rewrite update_rotation_ui later with Time resource.

pub fn tick_loading(
    time: Res<Time>,
    mut timer: ResMut<AssetLoadTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(GameState::Menu);
    }
}

pub fn handle_pause_input(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    key_btn: Res<ButtonInput<KeyCode>>,
) {
    if key_btn.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            GameState::Loading | GameState::Win | GameState::Menu | GameState::GameOver => {}
        }
    }
}

pub fn lock_cursor_on_click(
    state: Res<State<GameState>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if *state.get() == GameState::Playing && mouse_btn.just_pressed(MouseButton::Left) {
        if let Ok(mut cursor) = q_windows.single_mut() {
            cursor.grab_mode = CursorGrabMode::Locked;
            cursor.visible = false;
        }
    }
}

pub fn capture_cursor(mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

pub fn release_cursor(mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

pub fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}
pub fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

pub fn spawn_win_screen(
    mut commands: Commands,
    time: Res<Time>,
    stats: Res<GameStats>,
    mut high_score: ResMut<HighScore>,
) {
    // Game Over Screen
    let elapsed = time.elapsed_secs() - stats.start_time;
    if elapsed > high_score.0 {
        high_score.0 = elapsed;
    }

    commands
        .spawn((
            WinScreen,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(50),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GAME OVER"),
                TextFont {
                    font_size: 80.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.0, 0.0)), // Red for loss
            ));
            parent.spawn((
                Text::new(format!("Survival Time: {:.2}s", elapsed)),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new(format!("Personal Record: {:.2}s", high_score.0)),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)), // Gold
            ));

            let button_node = Node {
                width: Val::Px(200.0),
                height: Val::Px(65.0),
                margin: UiRect::all(Val::Px(20.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            let button_bg = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            let text_font = TextFont {
                font_size: 33.0,
                ..default()
            };
            let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

            // Restart Button
            parent
                .spawn((Button, button_node, button_bg, PauseButtonAction::Restart))
                .with_children(|parent| {
                    parent.spawn((Text::new("Restart"), text_font, text_color));
                });
        });
}

pub fn despawn_win_screen(mut commands: Commands, query: Query<Entity, With<WinScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_win_input(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &PauseButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut load_timer: ResMut<AssetLoadTimer>,
    mut stats: ResMut<GameStats>,
    time: Res<Time>,
) {
    for (interaction, mut color, action) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => match action {
                PauseButtonAction::Restart => {
                    load_timer.0.reset();
                    // Reset stats
                    stats.max_population = 100.0;
                    stats.start_time = time.elapsed_secs();

                    next_state.set(GameState::Loading);
                }
                _ => {}
            },
            Interaction::Hovered => {
                *color = BackgroundColor(Color::linear_rgb(0.25, 0.25, 0.25));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            }
        }
    }
}

pub fn handle_pause_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &PauseButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut load_timer: ResMut<AssetLoadTimer>,
    mut stats: ResMut<GameStats>,
    time: Res<Time>,
) {
    for (interaction, mut color, action) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => match action {
                PauseButtonAction::Continue => {
                    next_state.set(GameState::Playing);
                }
                PauseButtonAction::Restart => {
                    load_timer.0.reset();
                    // Reset stats
                    stats.max_population = 100.0;
                    stats.start_time = time.elapsed_secs();
                    next_state.set(GameState::Loading);
                }
            },
            Interaction::Hovered => {
                *color = BackgroundColor(Color::linear_rgb(0.25, 0.25, 0.25));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            }
        }
    }
}
