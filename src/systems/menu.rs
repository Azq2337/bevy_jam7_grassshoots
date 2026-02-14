use crate::resources::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct MenuScreen;

#[derive(Component)]
pub enum MenuButtonAction {
    PlayMergeToWin,
    PlaySurvival,
    ToggleDifficulty,
}

#[derive(Resource)]
pub struct SelectedDifficulty(pub u32); // Stores 8 to 16

pub fn spawn_menu_screen(mut commands: Commands, selected_difficulty: Res<SelectedDifficulty>) {
    commands
        .spawn((
            MenuScreen,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
            ZIndex(50),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("BEVY GEMINI"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::srgb(0.35, 0.6, 0.9)),
            ));

            parent.spawn((
                Text::new("Select Game Mode"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));

            // Difficulty Toggle
            let button_node = Node {
                width: Val::Px(300.0),
                height: Val::Px(50.0),
                border: UiRect::all(Val::Px(2.0)),
                margin: UiRect::all(Val::Px(5.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            let button_bg = BackgroundColor(Color::linear_rgb(0.2, 0.2, 0.2));
            let text_font = TextFont {
                font_size: 25.0,
                ..default()
            };

            parent
                .spawn((
                    Button,
                    button_node.clone(),
                    button_bg,
                    MenuButtonAction::ToggleDifficulty,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(format!("Target Level: {}", selected_difficulty.0)),
                        text_font.clone(),
                        TextColor(Color::srgb(1.0, 0.8, 0.2)),
                    ));
                });

            let play_button_node = Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(2.0)),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            let play_button_bg = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            let play_text_font = TextFont {
                font_size: 30.0,
                ..default()
            };
            let play_text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

            // Merge to Win Button
            parent
                .spawn((
                    Button,
                    play_button_node.clone(),
                    play_button_bg,
                    MenuButtonAction::PlayMergeToWin,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Mode 1: Merge to Win"),
                        play_text_font.clone(),
                        play_text_color,
                    ));
                });

            // Survival Button
            parent
                .spawn((
                    Button,
                    play_button_node,
                    play_button_bg,
                    MenuButtonAction::PlaySurvival,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Mode 2: Survival"),
                        play_text_font,
                        play_text_color,
                    ));
                });

            // Controls Info
            parent.spawn((
                Text::new(
                    "Controls: WASD to Move, SPACE to Jump, RMB to Grab/Aim, LMB to Throw/Shoot",
                ),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
        });
}

pub fn despawn_menu_screen(mut commands: Commands, query: Query<Entity, With<MenuScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_menu_interaction(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &mut BackgroundColor,
            &MenuButtonAction,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_mode: ResMut<GameMode>,
    mut selected_difficulty: ResMut<SelectedDifficulty>,
    assets: Res<GameAssets>,
) {
    for (_entity, interaction, mut color, action, children) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                commands.spawn(AudioPlayer::new(assets.click.clone()));

                match action {
                    MenuButtonAction::PlayMergeToWin => {
                        *game_mode = GameMode::MergeToWin {
                            target_level: selected_difficulty.0,
                        };
                        next_state.set(GameState::Playing);
                    }
                    MenuButtonAction::PlaySurvival => {
                        *game_mode = GameMode::Survival {
                            target_level: selected_difficulty.0,
                        };
                        next_state.set(GameState::Playing);
                    }
                    MenuButtonAction::ToggleDifficulty => {
                        selected_difficulty.0 += 1;
                        if selected_difficulty.0 > 16 {
                            selected_difficulty.0 = 8;
                        }
                        // Update text
                        for child in children {
                            if let Ok(mut text) = text_query.get_mut(*child) {
                                text.0 = format!("Target Level: {}", selected_difficulty.0);
                            }
                        }
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::linear_rgb(0.25, 0.25, 0.25));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            }
        }
    }
}
