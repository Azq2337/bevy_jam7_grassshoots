use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
        ))
        // 1. Initialize our Game State
        .init_state::<GameState>()
        
        // Setup runs once
        .add_systems(Startup, setup)
        
        // 2. These systems run ALL THE TIME
        .add_systems(Update, (handle_pause_input, lock_cursor_on_click))
        
        // 3. These systems ONLY run when Playing
        .add_systems(
            Update,
            (player_move, player_look).run_if(in_state(GameState::Playing)),
        )
        
        // 4. State Transition Systems (Fired automatically when state changes)
        .add_systems(OnEnter(GameState::Paused), (on_pause, spawn_pause_menu))
        .add_systems(OnExit(GameState::Paused), (on_resume, despawn_pause_menu))
        
        .run();
}

// --- STATES & COMPONENTS ---

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum GameState {
    #[default]
    Playing,
    Paused,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct CameraPitch(f32);

#[derive(Component)]
struct PauseMenu; // Marker for our UI overlay

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    // Lock cursor immediately on start
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 1.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.4, 0.2))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
    ));

    // Obstacle
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))),
        Transform::from_xyz(5.0, 1.0, -5.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 2.0, 4.0),
    ));

    // Lighting
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 0.0),
    ));

    // Player
    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.0),
        LockedAxes::ROTATION_LOCKED, 
        Friction::new(0.0),          
    )).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
            CameraPitch(0.0),
            Transform::from_xyz(0.0, 0.6, 0.0), 
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
}

// --- STATE MANAGEMENT ---

fn handle_pause_input(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    key_btn: Res<ButtonInput<KeyCode>>,
) {
    if key_btn.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
        }
    }
}

// If the player alt-tabs out and back in, left click re-locks the cursor
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

fn on_pause(
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
) {
    // 1. Free the mouse
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    // 2. Pause Bevy's internal clock (which automatically pauses Avian3D physics!)
    time.pause();
}

fn on_resume(
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
) {
    // 1. Recapture the mouse
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    // 2. Resume physics and time
    time.unpause();
}

// --- UI SYSTEMS ---

fn spawn_pause_menu(mut commands: Commands) {
    commands.spawn((
        PauseMenu, // Tag it so we can easily despawn it later
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute, // Overlay on top of everything
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)), // Dark transparent background
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

fn despawn_pause_menu(
    mut commands: Commands,
    query: Query<Entity, With<PauseMenu>>,
) {
    for entity in query.iter() {
        // despawn_recursive destroys the entity and all its children (the text)
        commands.entity(entity).despawn();
    }
}

// --- GAMEPLAY SYSTEMS (Unchanged, just gated by State) ---

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

fn player_look(
    mut mouse_motion: MessageReader<MouseMotion>, 
    primary_window: Query<&CursorOptions, With<PrimaryWindow>>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut CameraPitch), (With<Camera3d>, Without<Player>)>,
) {
    let Ok(cursor) = primary_window.single() else { return };
    if cursor.grab_mode == CursorGrabMode::None { return; }

    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }

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