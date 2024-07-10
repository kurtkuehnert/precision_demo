use bevy::color::palettes::basic;
use bevy::math::DVec3;
use bevy::{
    math::{DVec2, IVec2, Quat, Vec2},
    prelude::{Color, Gizmos},
};
use itertools::{iproduct, Itertools};

use crate::math::{Coordinate, SideParameter, TerrainModel, TerrainModelApproximation, Tile};

const DEBUG_SCALE: f32 = 1.0 / (1 << 5) as f32;
const ERROR_SCALE: f32 = 4.0;

pub fn draw_tile(
    gizmos: &mut Gizmos,
    model: &TerrainModel,
    tile: Tile,
    color: Color,
    offset: DVec3,
) {
    let size = 1.0 / Tile::tile_count(tile.lod) as f64;

    for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
        .into_iter()
        .map(|(x, y)| {
            let corner_st = IVec2::new(tile.xy.x + x, tile.xy.y + y).as_dvec2() * size;
            Coordinate::new(tile.side, corner_st).world_position(model, 0.0)
        })
        .tuple_windows()
    {
        gizmos
            .short_arc_3d_between(
                (model.position + offset).as_vec3(),
                (start + offset).as_vec3(),
                (end + offset).as_vec3(),
                color,
            )
            .resolution(20);
    }
}

pub fn draw_earth(gizmos: &mut Gizmos, model: &TerrainModel, lod: i32, offset: DVec3) {
    for (side, x, y) in iproduct!(0..6, 0..1 << lod, 0..1 << lod) {
        draw_tile(
            gizmos,
            model,
            Tile::new(side, lod, x, y),
            Color::BLACK,
            offset,
        )
    }
}

pub fn draw_origin(gizmos: &mut Gizmos, approximation: &TerrainModelApproximation, offset: DVec3) {
    let model = approximation.model.clone();

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
            ..
        },
    ) in approximation.sides.iter().enumerate()
    {
        let origin_position =
            Coordinate::new(side as u32, origin_st).world_position(&model, 0.0) + offset;

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
            0.0001 * model.scale() as f32,
            basic::OLIVE,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_s * DEBUG_SCALE,
            basic::YELLOW,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_t * DEBUG_SCALE,
            basic::GREEN,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_ss * DEBUG_SCALE,
            basic::RED,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_st * DEBUG_SCALE,
            basic::BLUE,
        );
        gizmos.arrow(
            origin_position.as_vec3(),
            origin_position.as_vec3() + c_tt * DEBUG_SCALE,
            basic::FUCHSIA,
        );

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_st = (origin_st
                    + DVec2::new(2.0 * x as f64 - 1.0, 2.0 * y as f64 - 1.0) * DEBUG_SCALE as f64)
                    .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                Coordinate::new(side as u32, corner_st).world_position(&model, 0.0)
            })
            .tuple_windows()
        {
            gizmos.short_arc_3d_between(
                (model.position + offset).as_vec3(),
                (start + offset).as_vec3(),
                (end + offset).as_vec3(),
                Color::WHITE,
            );
        }
    }
}

pub fn draw_error_field(
    gizmos: &mut Gizmos,
    approximation: &TerrainModelApproximation,
    offset: DVec3,
) {
    let count = 16;
    let side = approximation.view_coordinate.side;

    for (x, y) in iproduct!(-count..=count, -count..=count) {
        let relative_st = Vec2::new(x as f32, y as f32) / count as f32 * DEBUG_SCALE;

        let position =
            approximation.view_position + approximation.relative_position(relative_st, side);
        let approximate_position = approximation.view_position
            + approximation
                .approximate_relative_position(relative_st, side)
                .as_dvec3();

        let error = approximate_position - position;

        gizmos.arrow(
            (position + offset).as_vec3(),
            (position + offset).as_vec3() + error.as_vec3() * ERROR_SCALE,
            basic::RED,
        );
    }
}
