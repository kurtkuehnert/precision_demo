#![allow(dead_code, unused_variables)]

use bevy::{color::palettes::basic, math::DVec3, prelude::*, window::Cursor};
use precision_demo::{
    big_space::{
        BigSpaceCommands, BigSpacePlugin, GridTransformReadOnly, ReferenceFrame, ReferenceFrames,
    },
    camera::{DebugCameraBundle, DebugCameraController, DebugPlugin},
    draw::{draw_earth, draw_error_field, draw_origin, draw_tile},
    math::{TerrainModel, TerrainModelApproximation, Tile},
};

const RADIUS: f64 = 6371000.0;
const ORIGIN_LOD: i32 = 8;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        cursor: Cursor {
                            visible: false,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .build()
                .disable::<TransformPlugin>(),
            BigSpacePlugin::default(),
            DebugPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let model = TerrainModel::new(DVec3::new(0.0, 1.0, 1.0), 6378137.0, 6356752.314245);
    let camera_position = -DVec3::X * RADIUS * 3.0;

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let (earth_cell, earth_translation) = frame.translation_to_grid(model.position);
        let (camera_cell, camera_translation) = frame.translation_to_grid(camera_position);

        root.spawn_spatial((
            model,
            earth_cell,
            PbrBundle {
                transform: Transform::from_translation(earth_translation),
                mesh: meshes.add(Sphere::new(RADIUS as f32 * 0.4).mesh().ico(20).unwrap()),
                visibility: Visibility::Hidden,
                ..default()
            },
        ));

        root.spawn_spatial(DebugCameraBundle {
            camera: Camera3dBundle {
                transform: Transform::from_translation(camera_translation)
                    .looking_to(Vec3::X, Vec3::Y),
                projection: PerspectiveProjection {
                    near: 0.001,
                    ..default()
                }
                .into(),
                ..default()
            },
            cell: camera_cell,
            controller: DebugCameraController {
                translation_speed: RADIUS,
                ..default()
            },
            ..default()
        });
    });
}

fn update(
    mut view_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut show_error: Local<bool>,
    mut hide_origin: Local<bool>,
    mut gizmos: Gizmos,
    terrain_query: Query<(&TerrainModel, GridTransformReadOnly)>,
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
        *hide_origin = !*hide_origin;
    }

    if *freeze {
        return;
    }

    let (view, transform) = view_query.single();
    let frame = frames.parent_frame(view).unwrap();
    *view_position = transform.position_double(&frame);

    let (model, terrain_grid_transform) = terrain_query.single();
    let terrain_position = terrain_grid_transform.position_double(&frame);
    let offset = terrain_position - *view_position;

    dbg!(offset);

    let approximation =
        TerrainModelApproximation::compute(model.clone(), *view_position, ORIGIN_LOD);

    draw_earth(&mut gizmos, &model, 2, offset);

    if !*hide_origin {
        draw_origin(&mut gizmos, &approximation, offset);
    }
    if *show_error {
        draw_error_field(&mut gizmos, &approximation, offset);
    }

    {
        let xy = (Vec2::new(0.2483, 0.688143) * (1 << approximation.origin_lod) as f32).as_ivec2();
        let tile = Tile::new(0, approximation.origin_lod, xy.x, xy.y);
        let vertex_offset = Vec2::new(0.3754, 0.815768);

        let relative_st = approximation.relative_st(tile, vertex_offset);
        let relative_position = approximation.relative_position(relative_st, tile.side);
        let approximate_relative_st = approximation.approximate_relative_st(tile, vertex_offset);
        let approximate_relative_position =
            approximation.approximate_relative_position(approximate_relative_st, tile.side);

        let position = approximation.view_position + relative_position;
        let approximate_position =
            approximation.view_position + approximate_relative_position.as_dvec3();

        let error = position - approximate_position;

        // dbg!(error);

        draw_tile(&mut gizmos, &model, tile, basic::RED.into(), offset);

        gizmos.sphere(
            (position + offset).as_vec3(),
            Quat::IDENTITY,
            0.0001 * model.scale() as f32,
            basic::GREEN,
        );
        gizmos.sphere(
            (approximate_position + offset).as_vec3(),
            Quat::IDENTITY,
            0.0001 * model.scale() as f32,
            basic::RED,
        );
        gizmos.arrow(
            (position + offset).as_vec3(),
            (approximate_position + offset).as_vec3(),
            basic::RED,
        );
    }
}
