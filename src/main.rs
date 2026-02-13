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
            PhysicsPlugins::default(), // Initializes Avian3D Physics
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (player_move, player_look, cursor_grab))
        .run();
}

// --- COMPONENTS ---

#[derive(Component)]
struct Player;

#[derive(Component)]
struct CameraPitch(f32); // Stores our current up/down look angle

// --- SYSTEMS ---

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 1. A static floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(50.0, 1.0, 50.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.4, 0.2))),
        Transform::from_xyz(0.0, -0.5, 0.0),
        RigidBody::Static,
        Collider::cuboid(50.0, 1.0, 50.0),
    ));

    // 2. An obstacle to jump on
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.3, 0.3))),
        Transform::from_xyz(5.0, 1.0, -5.0),
        RigidBody::Static,
        Collider::cuboid(4.0, 2.0, 4.0),
    ));

    // 3. Lighting
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 8.0, 0.0),
    ));

    // 4. The Player (Physics Body)
    commands.spawn((
        Player,
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        Collider::capsule(0.4, 1.0),
        LockedAxes::ROTATION_LOCKED, // Prevents the capsule from falling over
        Friction::new(0.0),          // Prevents sticking to walls
    )).with_children(|parent| {
        // 5. The Camera (Child of the Player)
        parent.spawn((
            Camera3d::default(),
            CameraPitch(0.0),
            Transform::from_xyz(0.0, 0.6, 0.0), // Placed at "eye level"
        ));
    });

    // 6. UI Crosshair
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

fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Transform, &mut LinearVelocity), With<Player>>,
) {
    let Ok((transform, mut velocity)) = query.single_mut() else { return };

    // Get the player's local forward/right directions
    let forward = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction += forward; }
    if keyboard.pressed(KeyCode::KeyS) { direction -= forward; }
    if keyboard.pressed(KeyCode::KeyA) { direction -= right; }
    if keyboard.pressed(KeyCode::KeyD) { direction += right; }

    // Strip Y-axis intent so we don't accidentally try to walk into the sky/ground
    direction.y = 0.0;
    
    let movement_speed = 6.0;

    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        // Snappy movement: directly override the X and Z velocity
        velocity.x = direction.x * movement_speed;
        velocity.z = direction.z * movement_speed;
    } else {
        // Snappy stopping
        velocity.x = 0.0;
        velocity.z = 0.0;
    }

    // Jumping (Simple velocity check to prevent infinite mid-air jumps)
    if keyboard.just_pressed(KeyCode::Space) && velocity.y.abs() < 0.1 {
        velocity.y = 6.0;
    }
}

fn player_look(
    mut mouse_motion: MessageReader<MouseMotion>, 
    primary_window: Query<&CursorOptions, With<PrimaryWindow>>, // <-- CHANGED
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &mut CameraPitch), (With<Camera3d>, Without<Player>)>,
) {
    let Ok(cursor) = primary_window.single() else { return };
    
    // Only process look movement if the cursor is actually captured
    if cursor.grab_mode == CursorGrabMode::None {
        return; 
    }

    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
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

fn cursor_grab(
    mut q_windows: Query<&mut CursorOptions, With<PrimaryWindow>>, // <-- CHANGED
    mouse_btn: Res<ButtonInput<MouseButton>>,
    key_btn: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut cursor) = q_windows.single_mut() else { return };

    // Click left mouse to lock the cursor to the window
    if mouse_btn.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }

    // Press Escape to free it
    if key_btn.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}