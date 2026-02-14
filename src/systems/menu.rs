use crate::resources::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct MenuScreen;

#[derive(Component)]
pub enum MenuButtonAction {
    PlayMergeToWin,
    PlaySurvival,
}

pub fn spawn_menu_screen(mut commands: Commands) {
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

            let button_node = Node {
                width: Val::Px(300.0),
                height: Val::Px(65.0),
                border: UiRect::all(Val::Px(2.0)),
                margin: UiRect::all(Val::Px(10.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            };
            let button_bg = BackgroundColor(Color::linear_rgb(0.15, 0.15, 0.15));
            let text_font = TextFont {
                font_size: 30.0,
                ..default()
            };
            let text_color = TextColor(Color::srgb(0.9, 0.9, 0.9));

            // Merge to Win Button
            parent
                .spawn((
                    Button,
                    button_node.clone(),
                    button_bg,
                    MenuButtonAction::PlayMergeToWin,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Mode 1: Merge to Win"),
                        text_font.clone(),
                        text_color,
                    ));
                });

            // Survival Button
            parent
                .spawn((
                    Button,
                    button_node,
                    button_bg,
                    MenuButtonAction::PlaySurvival,
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new("Mode 2: Survival"), text_font, text_color));
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
    mut commands: Commands, // Not used but good practice
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_mode: ResMut<GameMode>,
) {
    for (interaction, mut color, action) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                match action {
                    MenuButtonAction::PlayMergeToWin => {
                        *game_mode = GameMode::MergeToWin;
                    }
                    MenuButtonAction::PlaySurvival => {
                        *game_mode = GameMode::Survival;
                    }
                }
                next_state.set(GameState::Playing);
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
