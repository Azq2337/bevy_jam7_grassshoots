use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    light::NotShadowCaster,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .insert_resource(ClearColor(Color::srgb(0.35, 0.6, 0.9)))
        .insert_resource(AssetLoadTimer(Timer::from_seconds(1.5, TimerMode::Once)))
        .insert_resource(GunTimer(Timer::from_seconds(0.15, TimerMode::Repeating)))
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_pause_input,
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
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(Update, tick_loading.run_if(in_state(GameState::Loading)))
        .add_systems(
            OnExit(GameState::Loading),
            (despawn_loading_screen, capture_cursor),
        )
        .add_systems(
            OnEnter(GameState::Paused),
            (release_cursor, pause_time, spawn_pause_menu),
        )
        .add_systems(
            OnExit(GameState::Paused),
            (capture_cursor, unpause_time, despawn_pause_menu),
        )
        .run();
}

// --- STATES, TIMERS, RESOURCES & COMPONENTS ---

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading,
    Playing,
    Paused,
}

#[derive(Resource)]
struct AssetLoadTimer(Timer);

#[derive(Resource)]
struct GunTimer(Timer);

#[derive(Resource)]
struct GameAssets {
    bullet_mesh: Handle<Mesh>,
    bullet_mat: Handle<StandardMaterial>,
    target_mesh: Handle<Mesh>,
    target_mat: Handle<StandardMaterial>,
    target_transparent_mat: Handle<StandardMaterial>,
}

#[derive(Component)]
struct Player {
    velocity: Vec3,
    grounded: bool,
}

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
    original_pos: Vec3,
}

#[derive(Component)]
struct RespawnTimer {
    timer: Timer,
    pos: Vec3,
}

#[derive(Component)]
struct Grabbed;

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let game_assets = GameAssets {
        bullet_mesh: meshes.add(Sphere::new(0.08)),
        bullet_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.0),
            emissive: LinearRgba::rgb(2.0, 1.5, 0.0).into(),
            ..default()
        }),
        target_mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        target_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.4, 0.1),
            perceptual_roughness: 0.5,
            ..default()
        }),
        target_transparent_mat: materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.4, 0.1, 0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    };

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
        commands.spawn((
            Target {
                health: 4,
                original_pos: pos,
            },
            Mesh3d(game_assets.target_mesh.clone()),
            MeshMaterial3d(game_assets.target_mat.clone()),
            Transform::from_translation(pos),
            RigidBody::Dynamic,
            Collider::cuboid(1.0, 1.0, 1.0),
        ));
    }

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

    let initial_pitch = -0.22;

    commands
        .spawn((
            Player {
                velocity: Vec3::ZERO,
                grounded: true,
            },
            DashCooldown(Timer::from_seconds(1.0, TimerMode::Once)),
            JumpCount(0),
            Transform::from_xyz(0.0, 2.0, 0.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)),
            RigidBody::Dynamic,
            Collider::capsule(0.4, 1.0),
            LockedAxes::ROTATION_LOCKED,
            Friction::new(0.0),
            // LinearDamping(10.0), // Removed global damping to fix gravity/jump
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
                    // SPWANS A SIMPLE GUN MODEL
                    cam.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.15, 0.15, 0.6))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.2, 0.2, 0.2),
                            perceptual_roughness: 0.1,
                            ..default()
                        })),
                        Transform::from_xyz(0.35, -0.25, -0.45),
                    ));
                });
        });

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
            GameState::Loading => {}
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            ZIndex(50),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED\nPress ESC to Resume"),
                TextFont {
                    font_size: 50.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
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
    let Ok((entity, transform, mut velocity, mut jump_count, mut dash_cd, mut dash_active)) =
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
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q_camera: Query<&mut Projection, With<Camera3d>>,
) {
    let Ok(mut projection) = q_camera.single_mut() else {
        return;
    };
    if let Projection::Perspective(ref mut persp) = *projection {
        let zoom_speed = 2.0 * time.delta_secs();
        if keyboard.pressed(KeyCode::PageUp) {
            persp.fov = (persp.fov + zoom_speed).min(120.0_f32.to_radians());
        }
        if keyboard.pressed(KeyCode::PageDown) {
            persp.fov = (persp.fov - zoom_speed).max(30.0_f32.to_radians());
        }
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
) {
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
        let duration = gun_timer.0.duration();
        gun_timer.0.set_elapsed(duration);
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
    mut target_q: Query<(Entity, &mut Target, &mut Transform)>,
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

        if let Ok((target_ent, mut target_data, mut target_transform)) = target_q.get_mut(target) {
            target_data.health -= 1;
            if target_data.health <= 0 {
                if let Ok(mut cmds) = commands.get_entity(target_ent) {
                    cmds.despawn();
                }
                commands.spawn(RespawnTimer {
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                    pos: Vec3::ZERO, // Randomization handled in handle_respawns
                });
            } else {
                target_transform.scale *= 0.75;
            }
        }
    }
}

fn handle_respawns(
    mut commands: Commands,
    time: Res<Time>,
    mut timers: Query<(Entity, &mut RespawnTimer)>,
    assets: Res<GameAssets>,
) {
    for (ent, mut respawn) in timers.iter_mut() {
        if respawn.timer.tick(time.delta()).just_finished() {
            commands.entity(ent).despawn();

            // Random position logic
            let x = rand::random::<f32>() * 40.0 - 20.0;
            let z = rand::random::<f32>() * 40.0 - 20.0;
            let random_pos = Vec3::new(x, 1.0, z);

            commands.spawn((
                Target {
                    health: 4,
                    original_pos: random_pos,
                },
                Mesh3d(assets.target_mesh.clone()),
                MeshMaterial3d(assets.target_mat.clone()),
                Transform::from_translation(random_pos),
                RigidBody::Dynamic,
                Collider::cuboid(1.0, 1.0, 1.0),
                SweptCcd::default(),
            ));
        }
    }
}

fn handle_grabbing(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    grabbed_q: Query<Entity, With<Grabbed>>,
    target_q: Query<Entity, With<Target>>,
    spatial_query: SpatialQuery, // Make mutable to use filters if needed, though CastRay might not need it mut unless we change pipeline. Actually SpatialQuery is a SystemParam.
    assets: Res<GameAssets>,
    player_q: Query<Entity, With<Player>>, // Needed to filter player
) {
    let Ok(cam_transform) = camera_q.single() else {
        return;
    };
    let player_ent = player_q.iter().next();

    if keyboard.just_pressed(KeyCode::KeyE) {
        if let Ok(grabbed_ent) = grabbed_q.single() {
            commands
                .entity(grabbed_ent)
                .remove::<Grabbed>()
                .remove::<Sensor>() // Remove Sensor so it collides again
                .insert(RigidBody::Dynamic)
                .insert(MeshMaterial3d(assets.target_mat.clone()));
        } else {
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
                        .insert(Sensor) // Make it a Sensor so it doesn't push the player
                        .insert(LinearVelocity::ZERO)
                        .insert(AngularVelocity::ZERO)
                        .insert(MeshMaterial3d(assets.target_transparent_mat.clone()));
                }
            }
        }
    }

    if keyboard.just_pressed(KeyCode::KeyQ) {
        if let Ok(grabbed_ent) = grabbed_q.single() {
            commands
                .entity(grabbed_ent)
                .remove::<Grabbed>()
                .remove::<Sensor>() // Remove Sensor
                .insert(RigidBody::Dynamic)
                .insert(LinearVelocity(cam_transform.forward() * 20.0))
                .insert(MeshMaterial3d(assets.target_mat.clone()));
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
