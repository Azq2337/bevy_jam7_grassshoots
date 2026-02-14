use crate::components::*;
use crate::resources::*;
use avian3d::prelude::*;
use bevy::prelude::*;

pub fn bullet_hit_target(
    mut commands: Commands,
    mut collision_events: bevy::ecs::message::MessageReader<CollisionStart>,
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
            // Play hit sound (or just reuse shoot/merge for now if no specific hit sound, sticking to basic plan)
            // commands.spawn(AudioPlayer::new(assets.hit.clone()));

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

                // Play destroy sound (spatial)
                // Use target transform for position
                commands.spawn((
                    AudioPlayer::new(assets.destroy.clone()),
                    PlaybackSettings::DESPAWN.with_spatial(true),
                    Transform::from_translation(target_transform.translation),
                ));

                commands.spawn(RespawnTimer {
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                });
            }
        }
    }
}

pub fn handle_respawns(
    mut commands: Commands,
    time: Res<Time>,
    mut timers: Query<(Entity, &mut RespawnTimer)>,
    assets: Res<GameAssets>,
    targets: Query<Entity, With<Target>>,
    stats: Res<GameStats>,
    listener_q: Query<&GlobalTransform, With<SpatialListener>>,
) {
    let listener_pos = listener_q.iter().next().map(|tr| tr.translation());

    // 1. Process timers
    for (ent, mut respawn) in timers.iter_mut() {
        if respawn.timer.tick(time.delta()).just_finished() {
            if let Ok(mut c) = commands.get_entity(ent) {
                c.despawn();
            }
            spawn_new_target(&mut commands, &assets, None, listener_pos);
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
            spawn_new_target(&mut commands, &assets, None, listener_pos);
        }
    }
}

pub fn spawn_new_target(
    commands: &mut Commands,
    assets: &GameAssets,
    data: Option<(Vec3, ShapeType, u32, i32)>, // pos, shape, level, health (u32, i32 matches struct)
    listener_pos: Option<Vec3>,
) {
    let (pos, shape, level, health) = if let Some((p, s, l, h)) = data {
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
        Magnetic { strength: 1.0 }, // Base strength, scaled by logic
    ));

    // Play spawn sound (spatial with cutoff)
    let volume = if let Some(l_pos) = listener_pos {
        let dist = pos.distance(l_pos);
        if dist > 3.0 {
            0.0
        } else {
            // Linear dropoff from 0 to 3m
            0.8 * (1.0 - (dist / 3.0)).clamp(0.0, 1.0)
        }
    } else {
        0.0 // Silent on startup
    };

    if volume > 0.0 {
        commands.spawn((
            AudioPlayer::new(assets.spawn.clone()),
            PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_volume(bevy::audio::Volume::Linear(volume)),
            Transform::from_translation(pos),
        ));
    }
}

pub fn handle_merging(
    mut commands: Commands,
    mut collision_events: bevy::ecs::message::MessageReader<CollisionStart>,
    target_q: Query<(&Transform, &Target, &Mergeable)>,
    assets: Res<GameAssets>,
    _game_mode: Res<GameMode>,
    listener_q: Query<&GlobalTransform, With<SpatialListener>>,
    grabbed_q: Query<Entity, With<Grabbed>>,
) {
    let mut processed = std::collections::HashSet::new();
    let listener_pos = listener_q.iter().next().map(|tr| tr.translation());
    let held_entity = grabbed_q.iter().next();

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
                    listener_pos,
                );

                // Check for held object priority
                let mut is_held_merge = false;
                if let Some(held) = held_entity {
                    if e1 == held || e2 == held {
                        is_held_merge = true;
                    }
                }

                // Play merge sound (spatial with cutoff)
                let volume = if is_held_merge {
                    1.0
                } else if let Some(l_pos) = listener_pos {
                    let dist = new_pos.distance(l_pos);
                    if dist > 3.0 {
                        0.0
                    } else {
                        1.0 * (1.0 - (dist / 3.0)).clamp(0.0, 1.0)
                    }
                } else {
                    0.0
                };

                if volume > 0.0 {
                    commands.spawn((
                        AudioPlayer::new(assets.merge.clone()),
                        PlaybackSettings::DESPAWN
                            .with_spatial(true)
                            .with_volume(bevy::audio::Volume::Linear(volume)),
                        Transform::from_translation(new_pos),
                    ));
                }
            }
        }
    }
}

pub fn update_gun(
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
    let ads_pos = Vec3::new(0.0, -0.20, -0.25); // Moved closer and slightly lower
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

pub fn handle_grabbing(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<&GlobalTransform, With<Camera3d>>,
    grabbed_q: Query<(Entity, &Mergeable), With<Grabbed>>,
    target_q: Query<Entity, With<Target>>,
    spatial_query: SpatialQuery,
    assets: Res<GameAssets>,
    player_q: Query<Entity, With<Player>>,
    mut shot_blocker: ResMut<ShotBlocker>,
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

            // Block shooting for a short time
            shot_blocker
                .0
                .set_duration(std::time::Duration::from_secs_f32(0.5));
            shot_blocker.0.reset();
        }
    }
}

pub fn update_grabbed_object(
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

pub fn tick_shot_blocker(mut shot_blocker: ResMut<ShotBlocker>, time: Res<Time>) {
    shot_blocker.0.tick(time.delta());
}

pub fn handle_oob(
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

pub fn check_game_over(
    mut next_state: ResMut<NextState<GameState>>,
    target_q: Query<&Mergeable>,
    game_mode: Res<GameMode>,
) {
    match *game_mode {
        GameMode::MergeToWin { target_level } => {
            // Reaching Target Level is a WIN
            for mergeable in target_q.iter() {
                if mergeable.level >= target_level {
                    next_state.set(GameState::Win);
                    return;
                }
            }
        }
        GameMode::Survival { target_level } => {
            // Reaching Target Level is a LOSE (Game Over)
            // Goal is to prevent merging to target
            for mergeable in target_q.iter() {
                if mergeable.level >= target_level {
                    next_state.set(GameState::GameOver);
                    return;
                }
            }
        }
    }
}

pub fn update_game_stats(mut stats: ResMut<GameStats>, time: Res<Time>) {
    stats.max_population = (stats.max_population + 50.0 * time.delta_secs()).min(800.0);
}

pub fn magnetic_pull(
    _commands: Commands,
    target_q: Query<(Entity, &Transform, &Mergeable, &RigidBody, &Magnetic)>,
    game_mode: Res<GameMode>,
    time: Res<Time>,
    mut velocities: Query<&mut LinearVelocity>,
) {
    if let GameMode::Survival { .. } = *game_mode {
        // N^2 but fine for now
        let targets: Vec<_> = target_q.iter().collect();

        for i in 0..targets.len() {
            let (_e1, t1, m1, _rb1, mag1) = targets[i];
            if m1.level < 4 {
                continue;
            }

            for j in 0..targets.len() {
                if i == j {
                    continue;
                }
                let (e2, t2, _m2, _rb2, _mag2) = targets[j];

                // Bigger pulls smaller or equal (based on level logic)
                // Actually, let's just use magnetic strength directly?
                // Logic: "Each object pulls others".
                // We use m1.level to gate if it pulls?

                let dir = t1.translation - t2.translation;
                let dist_sq = dir.length_squared();

                if dist_sq < 64.0 && dist_sq > 1.0 {
                    let force_mag = (m1.level as f32) * mag1.strength / dist_sq;
                    let force = dir.normalize() * force_mag * time.delta_secs() * 20.0;

                    if let Ok(mut vel) = velocities.get_mut(e2) {
                        vel.0 += force;
                    }
                }
            }
        }
    }
}
