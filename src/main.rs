#![allow(dead_code, unused_variables)]

use crate::big_space::{BigSpacePlugin, ReferenceFrame, ReferenceFrames};
use crate::camera::DebugCameraController;
use crate::draw::{draw_earth, draw_error_field, draw_origin, draw_tile};
use crate::{
    big_space::GridTransformReadOnly,
    camera::{DebugCameraBundle, DebugPlugin},
    math::{CameraParameter, Earth, Tile},
};
use ::big_space::BigSpaceCommands;
use bevy::color::palettes::basic;
use bevy::window::Cursor;
use bevy::{math::DVec3, prelude::*};

mod big_space;
mod camera;
mod draw;
mod math;

const RADIUS: f64 = 1.0; // 6371000.0;
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
    let earth = Earth::new(DVec3::new(0.0, 1.0, 1.0), RADIUS);
    let camera_position = -DVec3::X * RADIUS * 3.0;

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let (earth_cell, earth_translation) = frame.translation_to_grid(earth.position);
        let (camera_cell, camera_translation) = frame.translation_to_grid(camera_position);

        root
            .spawn_spatial((
            earth,
            earth_cell,
            PbrBundle {
                transform: Transform::from_translation(earth_translation),
                mesh: meshes.add(Sphere::new(RADIUS as f32 * 0.4).mesh().ico(20).unwrap()),
                visibility: Visibility::Hidden,
                ..default()
            },
        ));

        root
            .spawn_spatial(DebugCameraBundle {
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
    mut camera_position: Local<DVec3>,
    mut freeze: Local<bool>,
    mut show_error: Local<bool>,
    mut hide_origin: Local<bool>,
    mut gizmos: Gizmos,
    earth_query: Query<(&Earth, GridTransformReadOnly)>,
    camera_query: Query<(Entity, GridTransformReadOnly), With<Camera>>,
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

    let (camera, transform) = camera_query.single();
    let frame = frames.parent_frame(camera).unwrap();
    *camera_position = transform.position_double(&frame);

    let (&earth, earth_grid_transform) = earth_query.single();
    let earth_position = earth_grid_transform.position_double(&frame);
    let offset = earth_position - *camera_position;

    dbg!(offset);

    let camera = CameraParameter::compute(*camera_position, earth, ORIGIN_LOD);

    draw_earth(&mut gizmos, &earth, 2, offset);

    if !*hide_origin {
        draw_origin(&mut gizmos, &camera, offset);
    }
    if *show_error {
        draw_error_field(&mut gizmos, &camera, offset);
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

        // dbg!(error);

        draw_tile(&mut gizmos, &earth, tile, basic::RED.into(), offset);

        gizmos.sphere(
            (position + offset).as_vec3(),
            Quat::IDENTITY,
            0.0001 * earth.radius as f32,
            basic::GREEN,
        );
        gizmos.sphere(
            (approximate_position + offset).as_vec3(),
            Quat::IDENTITY,
            0.0001 * earth.radius as f32,
            basic::RED,
        );
        gizmos.arrow(
            (position + offset).as_vec3(),
            (approximate_position + offset).as_vec3(),
            basic::RED,
        );
    }
}
