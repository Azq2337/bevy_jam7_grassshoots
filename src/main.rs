use bevy::{
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
    input::mouse::MouseMotion,
    light::NotShadowCaster,
};
use avian3d::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.35, 0.6, 0.9)))
        // NEW: A 1.5 second timer to hide shader compilation stutter
        .insert_resource(AssetLoadTimer(Timer::from_seconds(1.5, TimerMode::Once)))
        
        // FIXED: Game now defaults to a Loading state
        .init_state::<GameState>() 
        .add_systems(Startup, setup)
        
        // Systems that run all the time
        .add_systems(
            Update,
            (handle_pause_input, lock_cursor_on_click, player_look, update_rotation_ui),
        )
        
        // Gameplay systems only run when Playing
        .add_systems(
            Update,
            (player_move, update_fov).run_if(in_state(GameState::Playing)),
        )
        
        // Loading State Transitions
        .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(Update, tick_loading.run_if(in_state(GameState::Loading)))
        .add_systems(OnExit(GameState::Loading), (despawn_loading_screen, capture_cursor))
        
        // Paused State Transitions
        .add_systems(OnEnter(GameState::Paused), (release_cursor, pause_time, spawn_pause_menu))
        .add_systems(OnExit(GameState::Paused), (capture_cursor, unpause_time, despawn_pause_menu))
        
        .run();
}

// --- STATES, TIMERS & COMPONENTS ---

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Loading, // Game starts here now!
    Playing,
    Paused,
}

#[derive(Resource)]
struct AssetLoadTimer(Timer);

#[derive(Component)]
struct Player;

#[derive(Component)]
struct CameraPitch(f32);

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct PauseMenu; 

#[derive(Component)]
struct RotationText; 

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // FIXED: Ground material tweaked for natural light scattering
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 1.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.2),
            perceptual_roughness: 0.9, // Very rough, spreads light naturally
            reflectance: 0.05,         // Low reflection
            ..default()
        })),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
    ));

    // FIXED: Obstacle material tweaked to be slightly shiny/glossy
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2), 
            perceptual_roughness: 0.35, // Smoother, catches sun highlights
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(5.0, 1.0, -5.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 2.0, 4.0),
    ));

    // --- CELESTIAL BODIES ---
    
    let sun_direction = Vec3::new(1.0, 1.0, 1.0).normalize();
    let moon_direction = Vec3::new(-1.0, 0.8, -1.0).normalize(); 
    let sky_distance = 150.0; 

    // Light Source
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(sun_direction * sky_distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Visual Sun
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

    // Visual Moon
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

    // --- TREES ---

    let trunk_mat = materials.add(Color::srgb(0.4, 0.2, 0.0));
    let trunk_mesh = meshes.add(Cylinder::new(0.5, 4.0));
    let leaves_mat = materials.add(Color::srgb(0.1, 0.4, 0.1));
    let leaves_mesh = meshes.add(Sphere::new(2.5));

    let tree_positions = [
        (15.0, -15.0), (-20.0, 10.0), (10.0, 20.0), (-15.0, -20.0),
        (20.0, -5.0), (-10.0, -15.0), (0.0, 22.0), (22.0, 5.0),
        (-22.0, -5.0), (-5.0, -22.0),
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

    // --- PLAYER & UI SETUP ---

    let initial_pitch = -0.22; 

    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 2.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)), 
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.0),
        LockedAxes::ROTATION_LOCKED, 
        Friction::new(0.0),          
    )).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
            // FIXED: Set the default perspective to 100 degrees FOV (converted to radians)
            Projection::Perspective(PerspectiveProjection {
                fov: 100.0_f32.to_radians(),
                ..default()
            }),
            CameraPitch(initial_pitch),
            Transform::from_xyz(0.0, 0.6, 0.0)
                .with_rotation(Quat::from_rotation_x(initial_pitch)), 
        ));
    });

    // Crosshair
    commands.spawn(Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Px(4.0),
                height: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ));
    });

    // Top-Left Rotation UI
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
    // Ticks the timer. Once 1.5 seconds pass, transition to Playing.
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
            GameState::Loading => {} // Do nothing if we press escape while loading
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

// Reusable Cursor Logic
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

// Reusable Time Logic
fn pause_time(mut time: ResMut<Time<Virtual>>) { time.pause(); }
fn unpause_time(mut time: ResMut<Time<Virtual>>) { time.unpause(); }


// --- UI SYSTEMS ---

fn spawn_loading_screen(mut commands: Commands) {
    commands.spawn((
        LoadingScreen, 
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute, 
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::BLACK), // Pure black to hide the stuttering game world
        ZIndex(100), // Forces the loading screen above all other UI elements
    )).with_children(|parent| {
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
    commands.spawn((
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
    )).with_children(|parent| {
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
    player_q: Query<&Transform, With<Player>>,
    camera_q: Query<(&CameraPitch, &Projection), With<Camera3d>>,
    mut text_q: Query<&mut Text, With<RotationText>>,
) {
    let Ok(player_transform) = player_q.single() else { return };
    let Ok((pitch, projection)) = camera_q.single() else { return };
    let Ok(mut text) = text_q.single_mut() else { return };

    let (yaw, _, _) = player_transform.rotation.to_euler(EulerRot::YXZ);
    
    let current_fov = match projection {
        Projection::Perspective(p) => p.fov.to_degrees(),
        _ => 0.0,
    };

    text.0 = format!(
        "Yaw: {:.1} deg\nPitch: {:.1} deg\nFOV: {:.1}", 
        yaw.to_degrees(), 
        pitch.0.to_degrees(),
        current_fov
    );
}

// --- GAMEPLAY SYSTEMS ---

fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Transform, &mut LinearVelocity), With<Player>>,
) {
    let Ok((transform, mut velocity)) = query.single_mut() else { return };

    let forward = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction += forward; }
    if keyboard.pressed(KeyCode::KeyS) { direction -= forward; }
    if keyboard.pressed(KeyCode::KeyA) { direction -= right; }
    if keyboard.pressed(KeyCode::KeyD) { direction += right; }

    direction.y = 0.0;
    let movement_speed = 6.0;

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        velocity.x = direction.x * movement_speed;
        velocity.z = direction.z * movement_speed;
    } else {
        velocity.x = 0.0;
        velocity.z = 0.0;
    }

    if keyboard.just_pressed(KeyCode::Space) && velocity.y.abs() < 0.1 {
        velocity.y = 6.0;
    }
}

fn update_fov(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q_camera: Query<&mut Projection, With<Camera3d>>,
) {
    let Ok(mut projection) = q_camera.single_mut() else { return };
    
    if let Projection::Perspective(ref mut persp) = *projection {
        let zoom_speed = 2.0 * time.delta_secs(); 
        
        // FIXED: Expanded constraints to prevent zooming out too far or in too close
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

    let Ok(cursor) = primary_window.single() else { return };
    if cursor.grab_mode == CursorGrabMode::None { return; }

    if delta == Vec2::ZERO { return; }

    let sensitivity = 0.002;

    if let Ok(mut player_transform) = player_query.single_mut() {
        player_transform.rotate_y(-delta.x * sensitivity);
    }

    if let Ok((mut camera_transform, mut pitch)) = camera_query.single_mut() {
        pitch.0 = (pitch.0 - delta.y * sensitivity).clamp(-1.54, 1.54);
        camera_transform.rotation = Quat::from_rotation_x(pitch.0);
    }
}