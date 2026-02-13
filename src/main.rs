use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    light::NotShadowCaster,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            PhysicsPlugins::default(),
        ))
        .insert_resource(ClearColor(Color::srgb(0.35, 0.6, 0.9)))
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (handle_pause_input, lock_cursor_on_click, player_look, update_rotation_ui),
        )
        .add_systems(
            Update,
            (player_move, update_fov).run_if(in_state(GameState::Playing)),
        )
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
struct PauseMenu; 

#[derive(Component)]
struct RotationText; 

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
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

    // Target Obstacle
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))),
        Transform::from_xyz(5.0, 1.0, -5.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 2.0, 4.0),
    ));

    // --- CELESTIAL BODIES ---
    
    let sun_direction = Vec3::new(1.0, 1.0, 1.0).normalize();
    // FIXED: Positive Y ensures the moon is actually in the sky, not underground!
    let moon_direction = Vec3::new(-1.0, 0.8, -1.0).normalize(); 
    let sky_distance = 150.0; 

    // 1. The Light Source
    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(sun_direction * sky_distance)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // 2. The Visual Sun
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(8.0))), 
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.2), // Yellow
            unlit: true, // Retains exact color without HDR blowout
            ..default()
        })),
        Transform::from_translation(sun_direction * sky_distance),
        NotShadowCaster, // FIXED: Prevents the giant round shadow
    ));

    // 3. The Visual Moon
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 1.0), // Pale blue
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

    // Random coordinates scattered around the map edges
    let tree_positions = [
        (15.0, -15.0), (-20.0, 10.0), (10.0, 20.0), (-15.0, -20.0),
        (20.0, -5.0), (-10.0, -15.0), (0.0, 22.0), (22.0, 5.0),
        (-22.0, -5.0), (-5.0, -22.0),
    ];

    for (x, z) in tree_positions {
        // Spawn Trunk (Has physics collider so you can't walk through it)
        commands.spawn((
            Mesh3d(trunk_mesh.clone()),
            MeshMaterial3d(trunk_mat.clone()),
            Transform::from_xyz(x, 2.0, z),
            RigidBody::Static,
            Collider::cylinder(0.5, 4.0),
        ));
        // Spawn Leaves (Just visual)
        commands.spawn((
            Mesh3d(leaves_mesh.clone()),
            MeshMaterial3d(leaves_mat.clone()),
            Transform::from_xyz(x, 5.0, z),
        ));
    }

    // --- PLAYER & UI SETUP ---

    let initial_pitch = -0.22; // -12.6 degrees down

    commands.spawn((
        Player,
        // FIXED: Explicitly set Yaw to -45 degrees to perfectly face the red block
        Transform::from_xyz(0.0, 2.0, 0.0)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_4)), 
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.0),
        LockedAxes::ROTATION_LOCKED, 
        Friction::new(0.0),          
    )).with_children(|parent| {
        parent.spawn((
            Camera3d::default(),
            CameraPitch(initial_pitch),
            // FIXED: Explicitly set Pitch to aim down at the block
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
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    time.pause();
}

fn on_resume(
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut time: ResMut<Time<Virtual>>,
) {
    if let Ok(mut cursor) = q_windows.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
    time.unpause();
}

// --- UI SYSTEMS ---

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
        
        if keyboard.pressed(KeyCode::PageUp) {
            persp.fov = (persp.fov + zoom_speed).min(2.5); 
        }
        if keyboard.pressed(KeyCode::PageDown) {
            persp.fov = (persp.fov - zoom_speed).max(0.5); 
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