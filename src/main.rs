#![allow(dead_code, unused_variables)]

use bevy::{math::DVec3, prelude::*};

use crate::{
    camera::{DebugCamera, DebugPlugin, DebugRig},
    draw::*,
    math::{CameraParameter, Earth, Tile},
};

mod camera;
mod draw;
mod math;

const RADIUS: f64 = 6371000.0;
const ORIGIN_LOD: i32 = 8;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DebugPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(DebugCamera::new(
        Vec3::new(-RADIUS as f32 * 2.0, 0.0, 0.0),
        -90.0,
        0.0,
        RADIUS as f32 / 2.0,
    ));
}

fn update(
    mut camera_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut show_error: Local<bool>,
    mut hide_origin: Local<bool>,
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<DebugRig>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyF) {
        *freeze = !*freeze;
    }
    if input.just_pressed(KeyCode::KeyE) {
        *show_error = !*show_error;
    }
    if input.just_pressed(KeyCode::KeyO) {
        *hide_origin = !*hide_origin;
    }

    if !*freeze {
        *camera_position = camera_query.single().translation.as_dvec3();
    }

    let earth = Earth {
        position: DVec3::new(8.0, 0.0, 3.0),
        radius: RADIUS,
    };

    let camera = CameraParameter::compute(*camera_position, earth, ORIGIN_LOD);

    draw_earth(&mut gizmos, &earth, 2);

    if !*hide_origin {
        draw_origin(&mut gizmos, &camera);
    }
    if *show_error {
        draw_error_field(&mut gizmos, &camera);
    }

    {
        let xy = (Vec2::new(0.2483, 0.688143) * (1 << camera.origin_lod) as f32).as_ivec2();
        let tile = Tile::new(0, camera.origin_lod, xy.x, xy.y);
        let vertex_offset = Vec2::new(0.3754, 0.815768);

        let relative_st = camera.relative_st(tile, vertex_offset);
        let relative_position = camera.relative_position(relative_st, tile.side);
        let approximate_relative_st = camera.approximate_relative_st(tile, vertex_offset);
        let approximate_relative_position =
            camera.approximate_relative_position(approximate_relative_st, tile.side);

        let position = camera.position + relative_position;
        let approximate_position = camera.position + approximate_relative_position.as_dvec3();

        let error = position - approximate_position;

        draw_tile(&mut gizmos, &earth, tile, Color::RED);

        gizmos.sphere(
            position.as_vec3(),
            Quat::IDENTITY,
            0.0001 * earth.radius as f32,
            Color::GREEN,
        );
        gizmos.sphere(
            approximate_position.as_vec3(),
            Quat::IDENTITY,
            0.0001 * earth.radius as f32,
            Color::RED,
        );
        gizmos.arrow(
            position.as_vec3(),
            approximate_position.as_vec3(),
            Color::RED,
        );
    }
}
