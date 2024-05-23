#![allow(dead_code, unused_variables)]

use bevy::{math::DVec2, prelude::*};
use bevy::math::DVec3;
use itertools::{iproduct, Itertools};

use crate::{
    camera::{DebugCamera, DebugPlugin, DebugRig},
    math::{CameraApproximationParams, SCALE, sphere_to_cube, Tile, world_position},
};
use crate::math::{CameraParams, SideCoefficients, THRESHOLD_LOD};

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

fn draw_threshold_area(gizmos: &mut Gizmos, camera_approximation: &CameraApproximationParams) {
    for (
        side,
        &SideCoefficients {
            delta_relative_st,
            origin_xy,
            c,
            c_s,
            c_t,
            c_ss,
            c_st,
            c_tt,
        },
    ) in camera_approximation.sides.iter().enumerate()
    {
        let origin_st = origin_xy.as_dvec2() / Tile::tile_count(THRESHOLD_LOD) as f64;
        let uv = sphere_to_cube(origin_st);
        let origin_position = world_position(side as u32, uv);

        gizmos.sphere(
            origin_position.as_vec3(),
            Quat::IDENTITY,
            0.0001,
            Color::GOLD,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_s,
            Color::YELLOW,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_t,
            Color::GREEN,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_ss,
            Color::RED,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_st,
            Color::BLUE,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_tt,
            Color::VIOLET,
        );

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_st = (origin_st + IVec2::new(x * 2 - 1, y * 2 - 1).as_dvec2() * SCALE)
                    .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                world_position(side as u32, sphere_to_cube(corner_st))
            })
            .tuple_windows()
        {
            gizmos.short_arc_3d_between(Vec3::ZERO, start.as_vec3(), end.as_vec3(), Color::WHITE);
        }
    }
}

fn update(
    mut cached_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<DebugRig>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let camera_transform = camera_query.single();
    let camera_position = camera_transform.translation.as_dvec3();

    if input.just_pressed(KeyCode::KeyF) {
        *freeze = !*freeze;
    }

    let camera_position = if *freeze {
        *cached_position
    } else {
        *cached_position = camera_position;
        camera_position
    };

    let camera = CameraParams::new(camera_position);
    let camera_approximation = CameraApproximationParams::compute(camera_position);

    let tile = Tile::new(0, THRESHOLD_LOD, 23, 12);
    let vertex_offset = Vec2::new(0.3754, 0.815768);

    draw_sphere(&mut gizmos, 2);
    draw_threshold_area(&mut gizmos, &camera_approximation);

    {
        let relative_st = camera.relative_st(tile, vertex_offset);
        let relative_position = camera.relative_position(relative_st, tile.side);

        let relative_st = camera_approximation.relative_st(tile, vertex_offset);
        let approximate_relative_position =
            camera_approximation.relative_position(relative_st, tile.side);

        let position = camera_position + relative_position;
        let approximate_position = camera_position + approximate_relative_position.as_dvec3();

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

    /*    {
        let count = 20;
        let side = 0;

        for (x, y) in iproduct!(-count..=count, -count..=count) {
            let relative_st = Vec2::new(x as f32, y as f32) / count as f32;

            let position = camera_position + camera.relative_position(relative_st, side);
            let approximate_position = camera_position
                + camera_approximation
                    .relative_position(relative_st, side)
                    .as_dvec3();

            let error = approximate_position - position;

            gizmos.arrow(
                position.as_vec3(),
                position.as_vec3() + error.as_vec3() * 1.0,
                Color::RED,
            );
        }
    }*/
}
