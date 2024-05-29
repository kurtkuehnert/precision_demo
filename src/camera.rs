//! This is a modified version of the big_space (https://github.com/aevyrie/big_space) camera controller.

use crate::big_space::{FloatingOrigin, GridCell, GridTransform, RootReferenceFrame};
use bevy::{
    input::mouse::MouseMotion,
    math::{DQuat, DVec3},
    prelude::*,
    transform::TransformSystem,
};

#[derive(Bundle)]
pub struct DebugCameraBundle {
    pub camera: Camera3dBundle,
    pub controller: DebugCameraController,
    pub cell: GridCell,
    pub origin: FloatingOrigin,
}

impl Default for DebugCameraBundle {
    fn default() -> Self {
        Self {
            camera: default(),
            controller: default(),
            cell: default(),
            origin: FloatingOrigin,
        }
    }
}

#[derive(Clone, Debug, Reflect, Component)]
pub struct DebugCameraController {
    pub enabled: bool,
    /// Smoothness of translation, from `0.0` to `1.0`.
    pub translational_smoothness: f64,
    /// Smoothness of rotation, from `0.0` to `1.0`.
    pub rotational_smoothness: f64,
    pub translation_speed: f64,
    pub rotation_speed: f64,
    pub acceleration_speed: f64,
    translation_velocity: DVec3,
    rotation_velocity: DQuat,
}

impl Default for DebugCameraController {
    fn default() -> Self {
        Self {
            enabled: false,
            translational_smoothness: 0.9,
            rotational_smoothness: 0.8,
            translation_speed: 10e6,
            rotation_speed: 1e-1,
            acceleration_speed: 4.0,
            translation_velocity: Default::default(),
            rotation_velocity: Default::default(),
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            camera_controller.before(TransformSystem::TransformPropagate),
        );
    }
}

pub fn camera_controller(
    space: Res<RootReferenceFrame>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_move: EventReader<MouseMotion>,
    mut camera: Query<(GridTransform, &mut DebugCameraController)>,
) {
    let (mut position, mut controller) = camera.single_mut();

    keyboard
        .just_pressed(KeyCode::KeyT)
        .then(|| controller.enabled = !controller.enabled);

    if !controller.enabled {
        return;
    }

    let mut rotation_direction = DVec3::ZERO; // x: pitch, y: yaw, z: roll
    let mut translation_direction = DVec3::ZERO; // x: left/right, y: up/down, z: forward/backward
    let mut acceleration = 0.0;

    if let Some(total_mouse_motion) = mouse_move.read().map(|e| e.delta).reduce(|sum, i| sum + i) {
        rotation_direction.x -= total_mouse_motion.y as f64;
        rotation_direction.y -= total_mouse_motion.x as f64;
    }

    keyboard
        .pressed(KeyCode::ArrowLeft)
        .then(|| translation_direction.x -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowRight)
        .then(|| translation_direction.x += 1.0);
    keyboard
        .pressed(KeyCode::PageUp)
        .then(|| translation_direction.y += 1.0);
    keyboard
        .pressed(KeyCode::PageDown)
        .then(|| translation_direction.y -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowUp)
        .then(|| translation_direction.z -= 1.0);
    keyboard
        .pressed(KeyCode::ArrowDown)
        .then(|| translation_direction.z += 1.0);
    keyboard.pressed(KeyCode::Home).then(|| acceleration -= 1.0);
    keyboard.pressed(KeyCode::End).then(|| acceleration += 1.0);

    let dt = time.delta_seconds_f64();
    let lerp_translation = 1.0 - controller.translational_smoothness.clamp(0.0, 0.999);
    let lerp_rotation = 1.0 - controller.rotational_smoothness.clamp(0.0, 0.999);
    let current_rotation = position.transform.rotation.as_dquat();

    controller.translation_speed *= 1.0 + acceleration * controller.acceleration_speed * dt;

    let translation_velocity_target =
        current_rotation * translation_direction * controller.translation_speed * dt;
    let rotation_velocity_target = DQuat::from_euler(
        EulerRot::XYZ,
        rotation_direction.x * controller.rotation_speed * dt,
        rotation_direction.y * controller.rotation_speed * dt,
        rotation_direction.z * controller.rotation_speed * dt,
    );

    controller.translation_velocity = controller
        .translation_velocity
        .lerp(translation_velocity_target, lerp_translation);
    controller.rotation_velocity = controller
        .rotation_velocity
        .slerp(rotation_velocity_target, lerp_rotation);

    let (cell_delta, translation_delta) =
        space.translation_to_grid(controller.translation_velocity);
    let rotation_delta = controller.rotation_velocity.as_quat();

    *position.cell += cell_delta;
    position.transform.translation += translation_delta;
    position.transform.rotation *= rotation_delta;
}
