use crate::components::*;
use crate::resources::*;
use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PrimaryWindow},
};

pub fn player_move(
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

pub fn player_look(
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

pub fn update_fov(
    time: Res<Time>,
    mut q_camera: Query<(&mut Projection, &GlobalTransform), With<Camera3d>>,
    mouse: Res<ButtonInput<MouseButton>>,
    grabbed_q: Query<Entity, With<Grabbed>>,
) {
    let Ok((mut projection, _)) = q_camera.single_mut() else {
        return;
    };
    if let Projection::Perspective(ref mut persp) = *projection {
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

pub fn handle_shooting(
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
    }
}

pub fn despawn_bullets(
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
