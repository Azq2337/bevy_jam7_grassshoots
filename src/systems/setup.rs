use crate::components::*;
use crate::resources::*;
use crate::systems::gameplay::spawn_new_target;
use avian3d::prelude::*;
use bevy::{light::NotShadowCaster, prelude::*};
use std::collections::HashMap;

pub fn setup(
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
            RigidBody::Static,
            Collider::cylinder(0.5, 4.0),
        ));
    }

    // Reticle
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

    // Control Guide (Top Right)
    commands.spawn((
        Text::new("Controls:\nWASD: Move\nSpace: Jump\nShift: Dash\nLMB: Shoot\nRMB: Aim / Grab Info\nRMB + Click: Grab\nLMB (Holding): Throw"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::FlexEnd,
            ..default()
        },
    ));
}

pub fn spawn_menu_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        MenuCamera,
    ));
}

pub fn despawn_menu_camera(mut commands: Commands, query: Query<Entity, With<MenuCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn cleanup_level(
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

pub fn spawn_level(
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
