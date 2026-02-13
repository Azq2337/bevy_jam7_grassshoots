use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    light::NotShadowCaster,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};
use std::collections::HashMap;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .insert_resource(ClearColor(Color::srgb(0.35, 0.6, 0.9)))
        .insert_resource(AssetLoadTimer(Timer::from_seconds(1.5, TimerMode::Once)))
        .insert_resource(GunTimer(Timer::from_seconds(0.15, TimerMode::Repeating)))
        .insert_resource(ShotBlocker(Timer::from_seconds(0.0, TimerMode::Once)))
        .insert_resource(GameStats {
            start_time: 0.0,
            max_population: 100.0,
        })
        .insert_resource(HighScore::default())
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_pause_input,
                handle_pause_buttons.run_if(in_state(GameState::Paused)),
                lock_cursor_on_click,
                player_look,
                update_rotation_ui,
            ),
        )
        .add_systems(
            Update,
            (
                player_move,
                update_fov,
                handle_shooting,
                despawn_bullets,
                bullet_hit_target,
                handle_respawns,
                handle_grabbing,
                update_grabbed_object,
                handle_merging,
                update_gun,
                handle_oob,
                check_game_over,
                update_game_stats,
                tick_shot_blocker,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, handle_win_input.run_if(in_state(GameState::Win)))
        .add_systems(
            OnEnter(GameState::Loading),
            (spawn_loading_screen, cleanup_level),
        )
        .add_systems(Update, tick_loading.run_if(in_state(GameState::Loading)))
        .add_systems(
            OnExit(GameState::Loading),
            (despawn_loading_screen, capture_cursor, spawn_level),
        )
        .add_systems(
            OnEnter(GameState::Paused),
            (release_cursor, pause_time, spawn_pause_menu),
        )
        .add_systems(
            OnExit(GameState::Paused),
            (capture_cursor, unpause_time, despawn_pause_menu),
        )
        .add_systems(OnEnter(GameState::Win), (release_cursor, spawn_win_screen))
        .add_systems(OnExit(GameState::Win), (despawn_win_screen, capture_cursor))
        .run();
}

// --- STATES, TIMERS, RESOURCES & COMPONENTS ---

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading,
    Playing,
    Paused,
    Win,
}

#[derive(Resource)]
struct AssetLoadTimer(Timer);

#[derive(Resource)]
struct GunTimer(Timer);

#[derive(Resource)]
struct GameAssets {
    bullet_mesh: Handle<Mesh>,
    bullet_mat: Handle<StandardMaterial>,
    target_transparent_mat: Handle<StandardMaterial>,
    shape_meshes: HashMap<ShapeType, Handle<Mesh>>,
    level_materials: Vec<Handle<StandardMaterial>>,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct DashCooldown(Timer);

#[derive(Component)]
struct DashActive(Timer);

#[derive(Component)]
struct JumpCount(u32);

#[derive(Component)]
struct CameraPitch(f32);

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct PauseMenu;

#[derive(Component)]
struct RotationText;

#[derive(Component)]
struct Bullet;

#[derive(Component)]
struct Lifetime(Timer);

#[derive(Component)]
struct Target {
    health: i32,
}

#[derive(Component)]
struct RespawnTimer {
    timer: Timer,
}

#[derive(Component)]
struct Grabbed;

#[derive(Component)]
struct Gun;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ShapeType {
    Cube,
    Sphere,
    Cylinder,
    Capsule,
    Cone,
    Torus,
    Tetrahedron,
}

#[derive(Resource)]
struct ShotBlocker(Timer);

#[derive(Resource)]
struct GameStats {
    start_time: f32,
    max_population: f32,
}

#[derive(Resource, Default)]
struct HighScore(f32);

#[derive(Component)]
struct Mergeable {
    shape: ShapeType,
    level: u32,
}

#[derive(Component)]
struct WinScreen;

#[derive(Component)]
enum PauseButtonAction {
    Continue,
    Restart,
}

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut shape_meshes = HashMap::new();
    shape_meshes.insert(ShapeType::Cube, meshes.add(Cuboid::new(1.0, 1.0, 1.0)));
    shape_meshes.insert(ShapeType::Sphere, meshes.add(Sphere::new(0.5)));
    shape_meshes.insert(ShapeType::Cylinder, meshes.add(Cylinder::new(0.5, 1.0)));
    shape_meshes.insert(ShapeType::Capsule, meshes.add(Capsule3d::new(0.5, 1.0)));
    shape_meshes.insert(ShapeType::Capsule, meshes.add(Capsule3d::new(0.5, 1.0)));
    shape_meshes.insert(ShapeType::Cone, meshes.add(Cone::new(0.5, 1.0)));
    shape_meshes.insert(ShapeType::Torus, meshes.add(Torus::new(0.3, 0.7)));
    shape_meshes.insert(ShapeType::Tetrahedron, meshes.add(Tetrahedron::default()));

    let mut level_materials = Vec::new();
    // Colors for levels 1-8 (Rainbow: Red -> Purple)
    let colors = [
        Color::srgb(1.0, 0.0, 0.0),   // 1 Red
        Color::srgb(1.0, 0.5, 0.0),   // 2 Orange
        Color::srgb(1.0, 1.0, 0.0),   // 3 Yellow
        Color::srgb(0.0, 1.0, 0.0),   // 4 Green (Spawn)
        Color::srgb(0.0, 0.0, 1.0),   // 5 Blue
        Color::srgb(0.29, 0.0, 0.51), // 6 Indigo
        Color::srgb(0.5, 0.0, 0.5),   // 7 Violet
        Color::srgb(0.0, 0.0, 0.0),   // 8 Black/Purple (Win)
    ];

    for color in colors {
        level_materials.push(materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.5,
            ..default()
        }));
    }

    let game_assets = GameAssets {
        bullet_mesh: meshes.add(Sphere::new(0.08)),
        bullet_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.0),
            emissive: LinearRgba::rgb(2.0, 1.5, 0.0).into(),
            ..default()
        }),
        target_transparent_mat: materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.4, 0.1, 0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        shape_meshes,
        level_materials,
    };

    commands.insert_resource(game_assets);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 1.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.2),
            perceptual_roughness: 0.9,
            reflectance: 0.05,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            perceptual_roughness: 0.35,
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(5.0, 1.0, -5.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 2.0, 4.0),
    ));

    let sun_direction = Vec3::new(1.0, 1.0, 1.0).normalize();
    let moon_direction = Vec3::new(-1.0, 0.8, -1.0).normalize();
    let sky_distance = 150.0;

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(sun_direction * sky_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(8.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.2),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(sun_direction * sky_distance),
        NotShadowCaster,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 1.0),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(moon_direction * sky_distance),
        NotShadowCaster,
    ));

    let trunk_mat = materials.add(Color::srgb(0.4, 0.2, 0.0));
    let trunk_mesh = meshes.add(Cylinder::new(0.5, 4.0));
    let leaves_mat = materials.add(Color::srgb(0.1, 0.4, 0.1));
    let leaves_mesh = meshes.add(Sphere::new(2.5));

    let tree_positions = [
        (15.0, -15.0),
        (-20.0, 10.0),
        (10.0, 20.0),
        (-15.0, -20.0),
        (20.0, -5.0),
        (-10.0, -15.0),
        (0.0, 22.0),
        (22.0, 5.0),
        (-22.0, -5.0),
        (-5.0, -22.0),
    ];

    for (x, z) in tree_positions {
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_mat.clone()),
            Transform::from_xyz(x, 2.0, z),
            RigidBody::Static,
            Collider::cylinder(0.5, 4.0),
        ));
        commands.spawn((
            Mesh3d(leaves_mesh.clone()),
            MeshMaterial3d(leaves_mat.clone()),
            Transform::from_xyz(x, 5.0, z),
        ));
    }

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
        });

    commands.spawn((
        Text::new("Yaw: 0.0 deg\nPitch: 0.0 deg\nFOV: 0.0"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        RotationText,
    ));
}

// --- STATE MANAGEMENT ---

fn tick_loading(
    time: Res<Time>,
    mut timer: ResMut<AssetLoadTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(GameState::Playing);
    }
}

fn handle_pause_input(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    key_btn: Res<ButtonInput<KeyCode>>,
) {
    if key_btn.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            GameState::Loading | GameState::Win => {}
        }
    }
}

fn lock_cursor_on_click(
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

fn capture_cursor(mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

fn release_cursor(mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

fn pause_time(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}
fn unpause_time(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}

// --- UI SYSTEMS ---

fn spawn_loading_screen(mut commands: Commands) {
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

fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn spawn_pause_menu(mut commands: Commands) {
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

fn despawn_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenu>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn update_rotation_ui(
    player_q: Query<(&Transform, &DashCooldown, Option<&DashActive>), With<Player>>,
    camera_q: Query<(&CameraPitch, &Projection), With<Camera3d>>,
    mut text_q: Query<&mut Text, With<RotationText>>,
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

    text.0 = format!(
        "Pos: {:.1}\nYaw: {:.1} deg\nPitch: {:.1} deg\nFOV: {:.1}\nDash: {}",
        player_transform.translation,
        yaw.to_degrees(),
        pitch.0.to_degrees(),
        current_fov,
        dash_status
    );
}

// --- GAMEPLAY SYSTEMS ---

fn player_move(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &Transform,
            &mut LinearVelocity,
            &mut JumpCount,
            &mut DashCooldown,
            Option<&mut DashActive>,
        ),
        With<Player>,
    >,
) {
    let Ok((entity, transform, mut velocity, mut jump_count, mut dash_cd, dash_active)) =
        query.single_mut()
    else {
        return;
    };

    // Ground Check
    let grounded = velocity.y.abs() < 0.1;
    if grounded {
        jump_count.0 = 0;
    }

    // Tick dash cooldown
    dash_cd.0.tick(time.delta());

    // Handle Active Dash
    if let Some(mut active) = dash_active {
        active.0.tick(time.delta());
        if active.0.just_finished() {
            commands.entity(entity).remove::<DashActive>();
            // Transition: Maybe set velocity to current velocity * 0.5?
            velocity.x *= 0.5;
            velocity.z *= 0.5;
        } else {
            // While dashing, maintain high velocity and ignore input
            return;
        }
    }

    let forward = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right;
    }

    direction.y = 0.0;
    let movement_speed = 6.0;

    // Only apply movement forces if we are NOT dashing (handled above)
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        let target_vel_x = direction.x * movement_speed;
        let target_vel_z = direction.z * movement_speed;

        // Manual acceleration/deceleration to prevent sliding while grounded
        velocity.x = target_vel_x;
        velocity.z = target_vel_z;
    } else if grounded {
        // Apply aggressive damping on ground when no input
        velocity.x *= 0.8; // Quick stop
        velocity.z *= 0.8;
    }
    // If in air, less control/damping?
    else {
        // Air control or air drag
        velocity.x *= 0.98;
        velocity.z *= 0.98;
    }

    // Ground Check (moved up)
    // let grounded = velocity.y.abs() < 0.1;
    // if grounded {
    //    jump_count.0 = 0;
    // }

    // Jump Logic
    if keyboard.just_pressed(KeyCode::Space) {
        if grounded {
            velocity.y = 6.0;
            jump_count.0 = 1;
        } else if jump_count.0 < 2 {
            velocity.y = 6.0;
            jump_count.0 += 1;
        }
    }

    // Dash Logic
    if keyboard.just_pressed(KeyCode::ShiftLeft) && dash_cd.0.elapsed() >= dash_cd.0.duration() {
        let dash_dir = if direction.length_squared() > 0.0 {
            direction
        } else {
            forward
        };
        velocity.x = dash_dir.x * 25.0; // High impulse
        velocity.z = dash_dir.z * 25.0;
        velocity.y = 0.5; // Slight hop
        dash_cd.0.reset();

        commands
            .entity(entity)
            .insert(DashActive(Timer::from_seconds(0.25, TimerMode::Once)));
    }
}

fn update_fov(
    time: Res<Time>,
    mut q_camera: Query<(&mut Projection, &GlobalTransform), With<Camera3d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    grabbed_q: Query<Entity, With<Grabbed>>,
) {
    let Ok((mut projection, _)) = q_camera.single_mut() else {
        return;
    };
    if let Projection::Perspective(ref mut persp) = *projection {
        // Target FOV:
        // If holding: Default (100 deg)
        // If ADS (RMB held & not holding): Zoom (e.g. 50 deg)
        // Default: 100 deg
        let target_fov_deg = if !grabbed_q.is_empty() {
            100.0
        } else if mouse.pressed(MouseButton::Right) {
            45.0 // ADS
        } else {
            100.0
        };

        // Lerp
        let current_fov_deg = persp.fov.to_degrees();
        let new_fov =
            current_fov_deg + (target_fov_deg - current_fov_deg) * 15.0 * time.delta_secs();
        persp.fov = new_fov.to_radians();
    }
}

fn player_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    primary_window: Query<&CursorOptions, With<PrimaryWindow>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut CameraPitch), (With<Camera3d>, Without<Player>)>,
) {
    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

    let Ok(cursor) = primary_window.single() else {
        return;
    };
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    if delta == Vec2::ZERO {
        return;
    }

    let sensitivity = 0.002;

    if let Ok(mut player_transform) = player_query.single_mut() {
        player_transform.rotate_y(-delta.x * sensitivity);
    }

    if let Ok((mut camera_transform, mut pitch)) = camera_query.single_mut() {
        pitch.0 = (pitch.0 - delta.y * sensitivity).clamp(-1.54, 1.54);
        camera_transform.rotation = Quat::from_rotation_x(pitch.0);
    }
}

// --- NEW COMBAT & INTERACTION SYSTEMS ---

fn handle_shooting(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut gun_timer: ResMut<GunTimer>,
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    assets: Res<GameAssets>,
    grabbed_q: Query<Entity, With<Grabbed>>,
    shot_blocker: Res<ShotBlocker>,
) {
    // Disable shooting if holding an object or blocked
    if !grabbed_q.is_empty() || shot_blocker.0.elapsed() < shot_blocker.0.duration() {
        return;
    }

    let Ok(cam_transform) = camera_q.single() else {
        return;
    };

    if mouse.pressed(MouseButton::Left) {
        gun_timer.0.tick(time.delta());
        if gun_timer.0.just_finished() {
            let forward = cam_transform.forward();
            let right = cam_transform.right();
            let up = cam_transform.up();

            let spawn_pos = cam_transform.translation() + forward * 1.0 + right * 0.4 - up * 0.3;

            commands.spawn((
                Mesh3d(assets.bullet_mesh.clone()),
                MeshMaterial3d(assets.bullet_mat.clone()),
                Transform::from_translation(spawn_pos),
                RigidBody::Dynamic,
                Collider::sphere(0.08),
                GravityScale(0.1),
                LinearVelocity(forward * 50.0),
                Bullet,
                Lifetime(Timer::from_seconds(2.0, TimerMode::Once)),
                // Use CCD to prevent tunneling at high speeds
                SweptCcd::default(),
                Restitution::new(0.8),  // Make bullets bouncy
                CollisionEventsEnabled, // Force collision events (Avian 0.5)
            ));
        }
    } else {
        // Cooldown or reset
        // let duration = gun_timer.0.duration();
        // gun_timer.0.set_elapsed(duration);
    }
}

fn despawn_bullets(
    mut commands: Commands,
    time: Res<Time>,
    mut bullets: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in bullets.iter_mut() {
        if lifetime.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_hit_target(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionStart>,
    bullet_q: Query<Entity, With<Bullet>>,
    mut target_q: Query<(
        Entity,
        &mut Target,
        &mut Transform,
        &mut Mergeable,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    assets: Res<GameAssets>,
) {
    for collision in collision_events.read() {
        let (bullet, target) = if bullet_q.contains(collision.collider1)
            && target_q.contains(collision.collider2)
        {
            (collision.collider1, collision.collider2)
        } else if bullet_q.contains(collision.collider2) && target_q.contains(collision.collider1) {
            (collision.collider2, collision.collider1)
        } else {
            continue;
        };

        if let Ok(mut cmds) = commands.get_entity(bullet) {
            cmds.despawn();
        }

        if let Ok((target_ent, _target_data, mut target_transform, mut mergeable, mut mat)) =
            target_q.get_mut(target)
        {
            // Decrement Level (Health)
            if mergeable.level > 1 {
                mergeable.level -= 1;
                // Update scale and material
                let base_scale = 0.5;
                let scale = base_scale * 1.3_f32.powf(mergeable.level as f32 - 1.0);
                target_transform.scale = Vec3::splat(scale);
                mat.0 = assets.level_materials
                    [(mergeable.level as usize - 1).min(assets.level_materials.len() - 1)]
                .clone();
            } else {
                // Destroy if Level 1
                if let Ok(mut cmds) = commands.get_entity(target_ent) {
                    cmds.despawn();
                }
                commands.spawn(RespawnTimer {
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                });
            }
        }
    }
}

fn handle_respawns(
    mut commands: Commands,
    time: Res<Time>,
    mut timers: Query<(Entity, &mut RespawnTimer)>,
    assets: Res<GameAssets>,
    targets: Query<Entity, With<Target>>,
    stats: Res<GameStats>,
) {
    // 1. Process timers
    for (ent, mut respawn) in timers.iter_mut() {
        if respawn.timer.tick(time.delta()).just_finished() {
            if let Ok(mut c) = commands.get_entity(ent) {
                c.despawn();
            }
            spawn_new_target(&mut commands, &assets, None);
        }
    }

    // 2. Maintain population
    // Spawn up to 5 per frame to catch up
    let current_count = targets.iter().len();
    let max_pop = stats.max_population as usize;

    if current_count < max_pop {
        let needed = max_pop - current_count;
        let to_spawn = needed.min(5);
        for _ in 0..to_spawn {
            spawn_new_target(&mut commands, &assets, None);
        }
    }
}

fn spawn_new_target(
    commands: &mut Commands,
    assets: &Res<GameAssets>,
    override_data: Option<(Vec3, ShapeType, u32, i32)>, // pos, shape, level, health
) {
    let (pos, shape, level, health) = if let Some((p, s, l, h)) = override_data {
        (p, s, l, h)
    } else {
        let x = rand::random::<f32>() * 40.0 - 20.0;
        let z = rand::random::<f32>() * 40.0 - 20.0;
        let shape = match rand::random::<u8>() % 5 {
            0 => ShapeType::Cube,
            1 => ShapeType::Sphere,
            2 => ShapeType::Cylinder,
            3 => ShapeType::Capsule,
            _ => ShapeType::Cone,
        };
        // Random spawn level 1-4
        let level = (rand::random::<u32>() % 4) + 1;
        (Vec3::new(x, 1.0, z), shape, level, 4)
    };

    let mesh = assets.shape_meshes.get(&shape).unwrap().clone();
    let material =
        assets.level_materials[(level as usize - 1).min(assets.level_materials.len() - 1)].clone();

    // Scale: Level 1=0.5, Growth factor 1.3
    let base_scale = 0.5;
    let scale = base_scale * 1.3_f32.powf(level as f32 - 1.0);

    let collider = match shape {
        ShapeType::Cube => Collider::cuboid(1.0, 1.0, 1.0),
        ShapeType::Sphere => Collider::sphere(0.5),
        ShapeType::Cylinder => Collider::cylinder(0.5, 1.0),
        ShapeType::Capsule => Collider::capsule(0.5, 1.0),
        ShapeType::Cone => Collider::cone(0.5, 1.0),
        _ => Collider::sphere(0.7), // Fallback/Torus/Tetrahedron
    };

    commands.spawn((
        Target { health },
        Mergeable { shape, level },
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
        RigidBody::Dynamic,
        collider,
        SweptCcd::default(),
        CollisionEventsEnabled,
        Restitution::new(0.5),
    ));
}

fn handle_merging(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionStart>,
    target_q: Query<(&Transform, &Target, &Mergeable)>,
    assets: Res<GameAssets>,
) {
    let mut processed = std::collections::HashSet::new();

    for collision in collision_events.read() {
        let e1 = collision.collider1;
        let e2 = collision.collider2;

        if processed.contains(&e1) || processed.contains(&e2) {
            continue;
        }

        if let (Ok((tr1, t1, m1)), Ok((tr2, t2, m2))) = (target_q.get(e1), target_q.get(e2)) {
            // Check if mergeable (same level, ignore shape)
            if m1.level == m2.level {
                let new_level = m1.level + 1;
                // Merge health? Or reset? Let's sum for now or max.
                let new_health = (t1.health + t2.health).min(10);
                let new_pos = (tr1.translation + tr2.translation) * 0.5;
                // Pick shape from one parent
                let new_shape = m1.shape;

                if let Ok(mut c) = commands.get_entity(e1) {
                    c.despawn();
                }
                if let Ok(mut c) = commands.get_entity(e2) {
                    c.despawn();
                }

                processed.insert(e1);
                processed.insert(e2);

                spawn_new_target(
                    &mut commands,
                    &assets,
                    Some((new_pos, new_shape, new_level, new_health)),
                );
            }
        }
    }
}

fn update_gun(
    time: Res<Time>,
    mut gun_q: Query<&mut Transform, With<Gun>>,
    mouse: Res<ButtonInput<MouseButton>>,
    grabbed_q: Query<Entity, With<Grabbed>>,
) {
    let Ok(mut transform) = gun_q.single_mut() else {
        return;
    };

    // Base positions (relative to camera)
    let default_pos = Vec3::new(0.35, -0.25, -0.45);
    let ads_pos = Vec3::new(0.0, -0.15, -0.3); // Centered
    let lowered_pos = Vec3::new(0.35, -0.5, -0.2); // Lowered

    // Rotation
    let default_rot = Quat::IDENTITY;
    let lowered_rot = Quat::from_rotation_x(-0.5); // Point down

    let (target_pos, target_rot) = if !grabbed_q.is_empty() {
        (lowered_pos, lowered_rot)
    } else if mouse.pressed(MouseButton::Right) {
        (ads_pos, default_rot)
    } else {
        (default_pos, default_rot)
    };

    transform.translation = transform
        .translation
        .lerp(target_pos, 15.0 * time.delta_secs());
    transform.rotation = transform
        .rotation
        .slerp(target_rot, 15.0 * time.delta_secs());
}

fn handle_grabbing(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    grabbed_q: Query<(Entity, &Mergeable), With<Grabbed>>,
    target_q: Query<Entity, With<Target>>,
    spatial_query: SpatialQuery,
    assets: Res<GameAssets>,
    player_q: Query<Entity, With<Player>>,
) {
    let Ok(cam_transform) = camera_q.single() else {
        return;
    };
    let player_ent = player_q.iter().next();

    // Grab or Drop with RMB
    if mouse.just_pressed(MouseButton::Right) {
        if let Some((grabbed_ent, mergeable)) = grabbed_q.iter().next() {
            // Drop
            let material = assets.level_materials
                [(mergeable.level as usize - 1).min(assets.level_materials.len() - 1)]
            .clone();
            commands
                .entity(grabbed_ent)
                .remove::<Grabbed>()
                .remove::<Sensor>()
                .insert(RigidBody::Dynamic)
                .insert(MeshMaterial3d(material));
        } else {
            // Try Grab
            let ray_origin = cam_transform.translation();
            let ray_dir = cam_transform.forward();
            let mut filter = SpatialQueryFilter::default();
            if let Some(p) = player_ent {
                filter = filter.with_excluded_entities([p]);
            }

            if let Some(hit) = spatial_query.cast_ray(ray_origin, ray_dir, 4.0, true, &filter) {
                if target_q.contains(hit.entity) {
                    commands
                        .entity(hit.entity)
                        .insert(Grabbed)
                        .insert(RigidBody::Kinematic)
                        .insert(Sensor)
                        .insert(LinearVelocity::ZERO)
                        .insert(AngularVelocity::ZERO)
                        .insert(MeshMaterial3d(assets.target_transparent_mat.clone()));
                }
            }
        }
    }

    // Throw with LMB
    if mouse.just_pressed(MouseButton::Left) {
        if let Some((grabbed_ent, mergeable)) = grabbed_q.iter().next() {
            let material = assets.level_materials
                [(mergeable.level as usize - 1).min(assets.level_materials.len() - 1)]
            .clone();
            commands
                .entity(grabbed_ent)
                .remove::<Grabbed>()
                .remove::<Sensor>()
                .insert(RigidBody::Dynamic)
                .insert(LinearVelocity(cam_transform.forward() * 20.0))
                .insert(MeshMaterial3d(material));
        }
    }
}

fn update_grabbed_object(
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    mut grabbed_q: Query<&mut Transform, With<Grabbed>>,
    time: Res<Time>,
) {
    let Ok(cam_transform) = camera_q.single() else {
        return;
    };
    if let Ok(mut transform) = grabbed_q.single_mut() {
        // Positioned slightly closer for a better FPS feel
        let hold_pos = cam_transform.translation() + cam_transform.forward() * 2.2;
        transform.translation = transform
            .translation
            .lerp(hold_pos, 20.0 * time.delta_secs());
    }
}

fn tick_shot_blocker(mut shot_blocker: ResMut<ShotBlocker>, time: Res<Time>) {
    shot_blocker.0.tick(time.delta());
}

fn handle_pause_buttons(
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

fn cleanup_level(
    mut commands: Commands,
    q_targets: Query<Entity, With<Target>>,
    q_player: Query<Entity, With<Player>>,
    q_bullets: Query<Entity, With<Bullet>>,
    q_respawns: Query<Entity, With<RespawnTimer>>,
    q_grabbed: Query<Entity, With<Grabbed>>,
) {
    for e in q_targets.iter() {
        commands.entity(e).despawn();
    }
    for e in q_player.iter() {
        commands.entity(e).despawn();
    }
    for e in q_bullets.iter() {
        commands.entity(e).despawn();
    }
    for e in q_respawns.iter() {
        commands.entity(e).despawn();
    }
    for e in q_grabbed.iter() {
        commands.entity(e).despawn();
    }
}

fn spawn_level(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let target_positions = [
        Vec3::new(3.0, 1.0, -8.0),
        Vec3::new(-4.0, 1.0, -7.0),
        Vec3::new(8.0, 1.0, -3.0),
        Vec3::new(-9.0, 1.0, -2.0),
        Vec3::new(6.0, 1.0, 4.0),
        Vec3::new(-7.0, 1.0, 5.0),
        Vec3::new(2.0, 1.0, 9.0),
        Vec3::new(-3.0, 1.0, 8.0),
        Vec3::new(10.0, 1.0, -10.0),
        Vec3::new(-10.0, 1.0, 10.0),
    ];

    for pos in target_positions {
        let shape = match rand::random::<u8>() % 5 {
            0 => ShapeType::Cube,
            1 => ShapeType::Sphere,
            2 => ShapeType::Cylinder,
            3 => ShapeType::Capsule,
            _ => ShapeType::Cone,
        };
        spawn_new_target(&mut commands, &assets, Some((pos, shape, 4, 4)));
    }

    let initial_pitch = -0.22;

    commands
        .spawn((
            Player,
            DashCooldown(Timer::from_seconds(1.0, TimerMode::Once)),
            JumpCount(0),
            Transform::from_xyz(0.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)),
            RigidBody::Dynamic,
            Collider::capsule(0.4, 1.0),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Camera3d::default(),
                    Projection::Perspective(PerspectiveProjection {
                        fov: 100.0_f32.to_radians(),
                        ..default()
                    }),
                    CameraPitch(initial_pitch),
                    Transform::from_xyz(0.0, 0.6, 0.0)
                        .with_rotation(Quat::from_rotation_x(initial_pitch)),
                ))
                .with_children(|cam| {
                    cam.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.15, 0.15, 0.6))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.2, 0.2, 0.2),
                            perceptual_roughness: 0.1,
                            ..default()
                        })),
                        Transform::from_xyz(0.35, -0.25, -0.45),
                        Gun,
                    ));
                });
        });
}

fn handle_oob(
    mut commands: Commands,
    mut player_q: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
    mut target_q: Query<(Entity, &mut Transform), (With<Target>, Without<Player>)>,
) {
    // Player OOB
    if let Some((_, mut transform, mut velocity)) = player_q.iter_mut().next() {
        if transform.translation.y < -10.0 {
            transform.translation = Vec3::new(0.0, 2.0, 0.0);
            *velocity = LinearVelocity::ZERO;
        }
    }

    // Target OOB
    for (entity, transform) in target_q.iter_mut() {
        if transform.translation.y < -10.0 {
            commands.entity(entity).despawn();
            commands.spawn(RespawnTimer {
                timer: Timer::from_seconds(3.0, TimerMode::Once),
            });
        }
    }
}

fn check_game_over(mut next_state: ResMut<NextState<GameState>>, target_q: Query<&Mergeable>) {
    // Level 16 is Game Over (Loss)
    for mergeable in target_q.iter() {
        if mergeable.level >= 8 {
            next_state.set(GameState::Win); // Using Win state as Game Over
        }
    }
}

fn update_game_stats(mut stats: ResMut<GameStats>, time: Res<Time>) {
    stats.max_population += 50.0 * time.delta_secs();
}

fn spawn_win_screen(
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

fn despawn_win_screen(mut commands: Commands, query: Query<Entity, With<WinScreen>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn handle_win_input(
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
