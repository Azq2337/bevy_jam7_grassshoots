use crate::components::*;
use crate::resources::*;
use avian3d::prelude::*;
use bevy::prelude::*;

use bevy::ecs::message::MessageReader;

pub fn player_look(
    mut player_q: Query<&mut Transform, With<Player>>,
    mut camera_q: Query<(&mut Transform, &mut CameraPitch), (With<Camera3d>, Without<Player>)>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    _time: Res<Time>,
    state: Res<State<GameState>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    let Some(mut player_transform) = player_q.iter_mut().next() else {
        return;
    };
    let Some((mut camera_transform, mut camera_pitch)) = camera_q.iter_mut().next() else {
        return;
    };

    let sensitivity = 0.003;

    for event in mouse_motion.read() {
        // Rotate player body around Y (yaw)
        player_transform.rotate_y(-event.delta.x * sensitivity);

        // Rotate camera around X (pitch)
        camera_pitch.0 -= event.delta.y * sensitivity;
        camera_pitch.0 = camera_pitch.0.clamp(-1.5, 1.5);

        camera_transform.rotation = Quat::from_rotation_x(camera_pitch.0);
    }
}

pub fn player_move(
    mut commands: Commands,
    mut player_q: Query<
        (
            Entity,
            &mut Transform,
            &mut LinearVelocity,
            &mut JumpCount,
            &mut DashCooldown,
            Option<&DashActive>,
            &mut FootstepTimer,
        ),
        With<Player>,
    >,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    spatial_query: SpatialQuery,
    assets: Res<GameAssets>,
) {
    let Some((
        entity,
        transform,
        mut velocity,
        mut jump_count,
        mut dash_cd,
        _dash_active,
        mut footstep_timer,
    )) = player_q.iter_mut().next()
    else {
        return;
    };

    // Ground check
    let ray_origin = transform.translation;
    let ray_dir = -Vec3::Y;
    let ray_dir_3 = Dir3::new(ray_dir).unwrap_or(Dir3::Y);

    let hit = spatial_query.cast_ray(
        ray_origin,
        ray_dir_3,
        1.1,
        true,
        &SpatialQueryFilter::from_excluded_entities([entity]),
    );
    let is_grounded = hit.is_some();

    if is_grounded {
        jump_count.0 = 0;
    }

    // Cooldowns
    dash_cd.0.tick(time.delta());

    // Movement
    let mut move_dir = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    if keys.pressed(KeyCode::KeyW) {
        move_dir += *forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_dir -= *forward;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_dir += *right;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_dir -= *right;
    }

    move_dir.y = 0.0;

    let is_moving = move_dir.length_squared() > 0.0;

    if is_moving {
        move_dir = move_dir.normalize();
    }

    // Footsteps
    if is_grounded && is_moving {
        footstep_timer.0.tick(time.delta());
        if footstep_timer.0.just_finished() {
            commands.spawn((
                AudioPlayer::new(assets.footstep.clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.5)),
            ));
        }
    } else {
        // Optional: Reset timer or keep it?
        // Keeping it allows continuing the "stride".
    }

    let speed = if keys.pressed(KeyCode::ShiftLeft) && dash_cd.0.elapsed() >= dash_cd.0.duration() {
        25.0
    } else {
        10.0
    };

    if keys.just_pressed(KeyCode::ShiftLeft) && dash_cd.0.elapsed() >= dash_cd.0.duration() {
        dash_cd.0.reset();

        let dash_dir = if is_moving {
            move_dir
        } else {
            // Use forward direction relative to flattened Y
            let fwd = transform.forward();
            let mut flat_fwd = Vec3::new(fwd.x, 0.0, fwd.z);
            if flat_fwd.length_squared() > 0.001 {
                flat_fwd = flat_fwd.normalize();
            } else {
                flat_fwd = Vec3::Z; // Fallback
            }
            flat_fwd
        };

        velocity.0 = dash_dir * 50.0;
        // Play dash sound (footstep)
        commands.spawn((
            AudioPlayer::new(assets.footstep.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(1.0)),
        ));
    } else {
        let target_vel = move_dir * speed;
        velocity.0.x = velocity.0.x.lerp(target_vel.x, 10.0 * time.delta_secs());
        velocity.0.z = velocity.0.z.lerp(target_vel.z, 10.0 * time.delta_secs());
    }

    if keys.just_pressed(KeyCode::Space) {
        if is_grounded || jump_count.0 < 1 {
            velocity.0.y = 7.0;
            jump_count.0 += 1;
            // Play jump sound (footstep)
            commands.spawn((
                AudioPlayer::new(assets.footstep.clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(0.8)),
            ));
        }
    }
}
// ... func split
// ... handle_shooting update below

pub fn update_fov(
    mut camera_q: Query<&mut Projection, With<Camera3d>>,
    player_q: Query<&LinearVelocity, With<Player>>,
    time: Res<Time>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Some(mut projection) = camera_q.iter_mut().next() else {
        return;
    };
    let Some(velocity) = player_q.iter().next() else {
        return;
    };

    if let Projection::Perspective(ref mut perspective) = *projection {
        let speed = velocity.0.length();

        let target_fov = if mouse.pressed(MouseButton::Right) {
            70.0_f32.to_radians()
        } else {
            100.0_f32.to_radians() + (speed * 0.005)
        };

        perspective.fov = perspective.fov.lerp(target_fov, 10.0 * time.delta_secs());
    }
}

pub fn handle_shooting(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    mut gun_timer: ResMut<GunTimer>,
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    assets: Res<GameAssets>,
    grabbed_q: Query<Entity, With<Grabbed>>, // Don't shoot if holding object
    time: Res<Time>,
    shot_blocker: Res<ShotBlocker>,
) {
    // If holding object, cannot shoot
    if !grabbed_q.is_empty() {
        return;
    }

    // If shot blocked (after throw)
    if shot_blocker.0.elapsed() < shot_blocker.0.duration() {
        return;
    }

    if mouse.pressed(MouseButton::Left) {
        gun_timer.0.tick(time.delta());
        if gun_timer.0.just_finished() {
            let Some(cam_transform) = camera_q.iter().next() else {
                return;
            };

            let spawn_pos =
                cam_transform.translation() + cam_transform.forward() * 0.8 - Vec3::Y * 0.2;
            let dir = cam_transform.forward();

            commands.spawn((
                Bullet,
                Mesh3d(assets.bullet_mesh.clone()),
                MeshMaterial3d(assets.bullet_mat.clone()),
                Transform::from_translation(spawn_pos),
                RigidBody::Dynamic,
                Collider::sphere(0.1),
                LinearVelocity(dir * 50.0),
                Lifetime(Timer::from_seconds(2.0, TimerMode::Once)),
                CollisionEventsEnabled, // Important for bullet hit detection
                SweptCcd::default(),
            ));

            // Play shoot sound
            commands.spawn((
                AudioPlayer::new(assets.shoot.clone()),
                PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::Linear(8.0)),
            ));
        }
    }
}

pub fn despawn_bullets(
    mut commands: Commands,
    mut q_bullets: Query<(Entity, &mut Lifetime), With<Bullet>>,
    time: Res<Time>,
) {
    for (entity, mut lifetime) in q_bullets.iter_mut() {
        if lifetime.0.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
    }
}
