use bevy::{
    math::{DVec2, IVec2, Quat, Vec2},
    prelude::{Color, Gizmos},
};
use itertools::{iproduct, Itertools};

use crate::math::{CameraParameter, Coordinate, Earth, SideParameter, Tile};

const DEBUG_SCALE: f32 = 1.0 / (1 << 5) as f32;
const ERROR_SCALE: f32 = 4.0;

pub(crate) fn draw_tile(gizmos: &mut Gizmos, earth: &Earth, tile: Tile, color: Color) {
    let size = 1.0 / Tile::tile_count(tile.lod) as f64;

    for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
        .into_iter()
        .map(|(x, y)| {
            let corner_st = IVec2::new(tile.xy.x + x, tile.xy.y + y).as_dvec2() * size;
            let local_position = Coordinate::new(tile.side, corner_st).to_local_position();
            earth.local_to_world(local_position)
        })
        .tuple_windows()
    {
        gizmos.short_arc_3d_between(
            earth.position.as_vec3(),
            start.as_vec3(),
            end.as_vec3(),
            color,
        );
    }
}

pub(crate) fn draw_earth(gizmos: &mut Gizmos, earth: &Earth, lod: i32) {
    for (side, x, y) in iproduct!(0..6, 0..1 << lod, 0..1 << lod) {
        draw_tile(gizmos, earth, Tile::new(side, lod, x, y), Color::BLACK)
    }
}

pub(crate) fn draw_origin(gizmos: &mut Gizmos, camera: &CameraParameter) {
    let earth = camera.earth;

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
        let local_position = Coordinate::new(side as u32, origin_st).to_local_position();
        let origin_position = earth.local_to_world(local_position);

        // if side as u32 == camera.coordinate.side {
        //     println!(
        //         "|c|: {}, |c_s|: {}, |c_t|: {}, |c_ss|: {}, |c_st|: {}, |c_tt|: {}",
        //         c.length(),
        //         c_s.length(),
        //         c_t.length(),
        //         c_ss.length(),
        //         c_st.length(),
        //         c_tt.length()
        //     );
        // }

        gizmos.sphere(
            origin_position.as_vec3(),
            Quat::IDENTITY,
            0.0001 * earth.radius as f32,
            Color::GOLD,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_s * DEBUG_SCALE,
            Color::YELLOW,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_t * DEBUG_SCALE,
            Color::GREEN,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_ss * DEBUG_SCALE,
            Color::RED,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_st * DEBUG_SCALE,
            Color::BLUE,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_tt * DEBUG_SCALE,
            Color::VIOLET,
        );

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_st = (origin_st
                    + DVec2::new(2.0 * x as f64 - 1.0, 2.0 * y as f64 - 1.0) * DEBUG_SCALE as f64)
                    .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                let local_position = Coordinate::new(side as u32, corner_st).to_local_position();
                earth.local_to_world(local_position)
            })
            .tuple_windows()
        {
            gizmos.short_arc_3d_between(
                earth.position.as_vec3(),
                start.as_vec3(),
                end.as_vec3(),
                Color::WHITE,
            );
        }
    }
}

pub(crate) fn draw_error_field(gizmos: &mut Gizmos, camera: &CameraParameter) {
    let count = 16;
    let side = camera.coordinate.side;

    for (x, y) in iproduct!(-count..=count, -count..=count) {
        let relative_st = Vec2::new(x as f32, y as f32) / count as f32 * DEBUG_SCALE;

        let position = camera.position + camera.relative_position(relative_st, side);
        let approximate_position = camera.position
            + camera
                .approximate_relative_position(relative_st, side)
                .as_dvec3();

        let error = approximate_position - position;

        gizmos.arrow(
            position.as_vec3(),
            position.as_vec3() + error.as_vec3() * ERROR_SCALE,
            Color::RED,
        );
    }
}
