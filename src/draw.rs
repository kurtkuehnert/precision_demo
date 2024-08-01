use bevy::{
    color::palettes::basic,
    math::{DVec2, DVec3, Quat},
    prelude::*,
};

use bevy_terrain::{
    math::{Coordinate, SurfaceApproximation, TileCoordinate},
    prelude::*,
};
use itertools::{iproduct, Itertools};

const DEBUG_SCALE: f32 = 1.0 / (1 << 5) as f32;
const ERROR_SCALE: f32 = 4.0;

pub fn draw_tile(
    gizmos: &mut Gizmos,
    model: &TerrainModel,
    tile: TileCoordinate,
    color: Color,
    offset: DVec3,
) {
    let size = 1.0 / TileCoordinate::count(tile.lod) as f64;

    for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
        .into_iter()
        .map(|(x, y)| {
            let corner_st = UVec2::new(tile.x + x, tile.y + y).as_dvec2() * size;
            Coordinate::new(tile.face, corner_st).world_position(model, 0.0)
        })
        .tuple_windows()
    {
        gizmos
            .short_arc_3d_between(
                (model.position() + offset).as_vec3(),
                (start + offset).as_vec3(),
                (end + offset).as_vec3(),
                color,
            )
            .resolution(20);
    }
}

pub fn draw_earth(gizmos: &mut Gizmos, model: &TerrainModel, lod: u32, offset: DVec3) {
    for (face, x, y) in iproduct!(0..6, 0..1 << lod, 0..1 << lod) {
        draw_tile(
            gizmos,
            model,
            TileCoordinate::new(face, lod, x, y),
            Color::BLACK,
            offset,
        )
    }
}

pub fn draw_approximation(
    gizmos: &mut Gizmos,
    model: &TerrainModel,
    view_coordinates: &[Coordinate],
    approximations: &[SurfaceApproximation],
    offset: DVec3,
) {
    for face in 0..model.face_count() {
        let &SurfaceApproximation {
            c,
            c_du,
            c_dv,
            c_duu,
            c_duv,
            c_dvv,
        } = &approximations[face as usize];

        let view_coordinate = view_coordinates[face as usize];
        let view_position = view_coordinate.world_position(&model, 0.0) + offset;

        gizmos.sphere(
            view_position.as_vec3(),
            Quat::IDENTITY,
            0.0001 * model.scale() as f32,
            basic::OLIVE,
        );
        gizmos.arrow(
            view_position.as_vec3(),
            view_position.as_vec3() + c_du * DEBUG_SCALE,
            basic::YELLOW,
        );
        gizmos.arrow(
            view_position.as_vec3(),
            view_position.as_vec3() + c_dv * DEBUG_SCALE,
            basic::GREEN,
        );
        gizmos.arrow(
            view_position.as_vec3(),
            view_position.as_vec3() + c_duu * DEBUG_SCALE,
            basic::RED,
        );
        gizmos.arrow(
            view_position.as_vec3(),
            view_position.as_vec3() + c_duv * DEBUG_SCALE,
            basic::BLUE,
        );
        gizmos.arrow(
            view_position.as_vec3(),
            view_position.as_vec3() + c_dvv * DEBUG_SCALE,
            basic::FUCHSIA,
        );

        for (start, end) in [(0, 0), (0, 1), (1, 1), (1, 0), (0, 0)]
            .into_iter()
            .map(|(x, y)| {
                let corner_uv = (view_coordinate.uv
                    + DVec2::new(2.0 * x as f64 - 1.0, 2.0 * y as f64 - 1.0) * DEBUG_SCALE as f64)
                    .clamp(DVec2::splat(0.0), DVec2::splat(1.0));
                Coordinate::new(face as u32, corner_uv).world_position(&model, 0.0)
            })
            .tuple_windows()
        {
            gizmos.short_arc_3d_between(
                (model.position() + offset).as_vec3(),
                (start + offset).as_vec3(),
                (end + offset).as_vec3(),
                Color::WHITE,
            );
        }
    }
}
