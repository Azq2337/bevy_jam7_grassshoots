use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct DashCooldown(pub Timer);

#[derive(Component)]
pub struct DashActive;//(pub Timer);

#[derive(Component)]
pub struct JumpCount(pub u32);

#[derive(Component)]
pub struct CameraPitch(pub f32);

#[derive(Component)]
pub struct LoadingScreen;

#[derive(Component)]
pub struct PauseMenu;

#[derive(Component)]
pub struct RotationText;

#[derive(Component)]
pub struct Bullet;

#[derive(Component)]
pub struct Lifetime(pub Timer);

#[derive(Component)]
pub struct Target {
    pub health: i32,
}

#[derive(Component)]
pub struct RespawnTimer {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Grabbed;

#[derive(Component)]
pub struct Gun;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeType {
    Cube,
    Sphere,
    Cylinder,
    Capsule,
    Cone,
    Torus,
    Tetrahedron,
}

#[derive(Component)]
pub struct Mergeable {
    pub shape: ShapeType,
    pub level: u32,
}

#[derive(Component)]
pub struct WinScreen;

#[derive(Component, PartialEq, Eq, Copy, Clone)]
pub enum PauseButtonAction {
    Continue,
    Restart,
}

#[derive(Component)]
pub struct FootstepTimer(pub Timer);

#[derive(Component)]
pub struct Magnetic {
    pub strength: f32,
}

#[derive(Component)]
pub struct MenuCamera;
