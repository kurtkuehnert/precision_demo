#![allow(dead_code, unused_variables)]

use bevy::{
    math::{DVec2, DVec3},
    prelude::*,
};
use itertools::{iproduct, Itertools};

use crate::{
    camera::{DebugCamera, DebugPlugin, DebugRig},
    math::{CameraParameter, Coordinate, SideParameter, sphere_to_cube, Tile, world_position},
};
use crate::math::THRESHOLD_LOD;

mod camera;
mod math;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DebugPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(DebugCamera::default());
}

fn draw_tile(gizmos: &mut Gizmos, tile: Tile, color: Color) {
    let size = 1.0 / Tile::tile_count(tile.lod) as f64;

    for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
        .into_iter()
        .map(|(x, y)| {
            let corner_st = IVec2::new(tile.xy.x + x, tile.xy.y + y).as_dvec2() * size;
            world_position(tile.side, sphere_to_cube(corner_st))
        })
        .tuple_windows()
    {
        gizmos.short_arc_3d_between(Vec3::ZERO, start.as_vec3(), end.as_vec3(), color);
    }
}

fn draw_sphere(gizmos: &mut Gizmos, lod: i32) {
    for (side, x, y) in iproduct!(0..6, 0..1 << lod, 0..1 << lod) {
        draw_tile(gizmos, Tile::new(side, lod, x, y), Color::BLACK)
    }
}

fn draw_origin(gizmos: &mut Gizmos, camera: &CameraParameter) {
    for (
        side,
        &SideParameter {
            origin_xy,
            origin_st,
            delta_relative_st,
            c,
            c_s,
            c_t,
            c_ss,
            c_st,
            c_tt,
        },
    ) in camera.sides.iter().enumerate()
    {
        let origin_position = world_position(side as u32, sphere_to_cube(origin_st));

        let scale = 0.01;

        gizmos.sphere(
            origin_position.as_vec3(),
            Quat::IDENTITY,
            0.0001,
            Color::GOLD,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_s * scale,
            Color::YELLOW,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_t * scale,
            Color::GREEN,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_ss * scale,
            Color::RED,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_st * scale,
            Color::BLUE,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_tt * scale,
            Color::VIOLET,
        );

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_st = (origin_st
                    + IVec2::new(x * 2 - 1, y * 2 - 1).as_dvec2() * scale as f64)
                    .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                world_position(side as u32, sphere_to_cube(corner_st))
            })
            .tuple_windows()
        {
            gizmos.short_arc_3d_between(Vec3::ZERO, start.as_vec3(), end.as_vec3(), Color::WHITE);
        }
    }
}

fn draw_error_field(gizmos: &mut Gizmos, camera: &CameraParameter) {
    let scale = (1 << 4) as f32;
    let count = 16;
    let side = Coordinate::from_world_position(camera.world_position).side;
    let error_scale = 4.0;

    for (x, y) in iproduct!(-count..=count, -count..=count) {
        let relative_st = Vec2::new(x as f32, y as f32) / count as f32 / scale;

        let position = camera.world_position + camera.relative_position(relative_st, side);
        let approximate_position = camera.world_position
            + camera
                .approximate_relative_position(relative_st, side)
                .as_dvec3();

        let error = approximate_position - position;

        gizmos.arrow(
            position.as_vec3(),
            position.as_vec3() + error.as_vec3() * error_scale,
            Color::RED,
        );
    }
}

fn update(
    mut camera_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<DebugRig>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyF) {
        *freeze = !*freeze;
    }

    if !*freeze {
        *camera_position = camera_query.single().translation.as_dvec3();
    }

    let camera = CameraParameter::compute(*camera_position);

    draw_sphere(&mut gizmos, 2);
    draw_origin(&mut gizmos, &camera);
    draw_error_field(&mut gizmos, &camera);

    {
        let tile = Tile::new(0, THRESHOLD_LOD, 23, 12);
        let vertex_offset = Vec2::new(0.3754, 0.815768);

        let relative_st = camera.relative_st(tile, vertex_offset);
        let relative_position = camera.relative_position(relative_st, tile.side);
        let approximate_relative_st = camera.approximate_relative_st(tile, vertex_offset);
        let approximate_relative_position =
            camera.approximate_relative_position(approximate_relative_st, tile.side);

        let position = camera.world_position + relative_position;
        let approximate_position = camera.world_position + approximate_relative_position.as_dvec3();

        let error = position - approximate_position;

        draw_tile(&mut gizmos, tile, Color::RED);

        gizmos.sphere(position.as_vec3(), Quat::IDENTITY, 0.0001, Color::GREEN);
        gizmos.sphere(
            approximate_position.as_vec3(),
            Quat::IDENTITY,
            0.0001,
            Color::RED,
        );
        gizmos.arrow(
            position.as_vec3(),
            approximate_position.as_vec3(),
            Color::RED,
        );
    }
}
