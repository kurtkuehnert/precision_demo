#![allow(dead_code, unused_variables)]

use bevy::{math::DVec3, prelude::*};
use bevy_terrain::{
    big_space::{GridTransformReadOnly, ReferenceFrames},
    math::{Coordinate, SurfaceApproximation},
    prelude::*,
};
use itertools::Itertools;
use precision_demo::draw::{draw_approximation, draw_earth};

const RADIUS: f64 = 6371000.0;
const ORIGIN_LOD: i32 = 8;

#[derive(Component)]
struct Model(TerrainModel);

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            TerrainPlugin,
            TerrainDebugPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    let model = TerrainModel::ellipsoid(
        DVec3::new(0.0, 1.0, 1.0),
        6378137.0,
        6356752.314245,
        0.0,
        0.0,
    );

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let (earth_cell, earth_translation) = frame.translation_to_grid(model.position());

        root.spawn_spatial((
            Model(model),
            earth_cell,
            Transform::from_translation(earth_translation),
        ));

        root.spawn_spatial(DebugCameraBundle::new(
            -DVec3::X * RADIUS * 3.0,
            RADIUS,
            &frame,
        ));
    });
}

fn update(
    mut view_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut show_error: Local<bool>,
    mut hide_approximation: Local<bool>,
    mut gizmos: Gizmos,
    terrain_query: Query<(&Model, GridTransformReadOnly)>,
    view_query: Query<(Entity, GridTransformReadOnly), With<Camera>>,
    input: Res<ButtonInput<KeyCode>>,
    frames: ReferenceFrames,
) {
    if input.just_pressed(KeyCode::KeyF) {
        *freeze = !*freeze;
    }
    if input.just_pressed(KeyCode::KeyE) {
        *show_error = !*show_error;
    }
    if input.just_pressed(KeyCode::KeyO) {
        *hide_approximation = !*hide_approximation;
    }

    if *freeze {
        return;
    }

    let (view, transform) = view_query.single();
    let frame = frames.parent_frame(view).unwrap();
    *view_position = transform.position_double(&frame);

    let (Model(model), terrain_grid_transform) = terrain_query.single();
    let terrain_position = terrain_grid_transform.position_double(&frame);
    let offset = terrain_position - *view_position;

    let view_coordinate = Coordinate::from_world_position(*view_position, model);

    let view_coordinates = (0..6)
        .map(|face| view_coordinate.project_to_face(face, model))
        .collect_vec();

    let approximations = view_coordinates
        .iter()
        .map(|&view_coordinate| {
            SurfaceApproximation::compute(view_coordinate, *view_position, model)
        })
        .collect_vec();

    draw_earth(&mut gizmos, model, 2, offset);

    if !*hide_approximation {
        draw_approximation(
            &mut gizmos,
            model,
            &view_coordinates,
            &approximations,
            offset,
        );
    }
}
