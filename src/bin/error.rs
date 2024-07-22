use bevy::{
    color::palettes::basic,
    math::{DVec2, DVec3},
    prelude::*,
};
use bevy_terrain::{
    math::{Coordinate, SurfaceApproximation, TileCoordinate, ViewCoordinate},
    prelude::*,
};
use itertools::Itertools;
use precision_demo::draw::draw_earth;
use rand::{prelude::ThreadRng, thread_rng, Rng};

const C_SQR: f32 = 0.87 * 0.87;

fn f32_world_position((tile, tile_uv): (TileCoordinate, Vec2), model: &TerrainModel) -> DVec3 {
    let uv =
        (UVec2::new(tile.x, tile.y).as_vec2() + tile_uv) / TileCoordinate::count(tile.lod) as f32;

    let w = (uv - 0.5) / 0.5;
    let uv = w / (1.0 + C_SQR - C_SQR * w * w).powf(0.5);

    let local_position = match tile.side {
        0 => Vec3::new(-1.0, -uv.y, uv.x),
        1 => Vec3::new(uv.x, -uv.y, 1.0),
        2 => Vec3::new(uv.x, 1.0, uv.y),
        3 => Vec3::new(1.0, -uv.x, uv.y),
        4 => Vec3::new(uv.y, -uv.x, -1.0),
        5 => Vec3::new(uv.y, -1.0, uv.x),
        _ => unreachable!(),
    }
    .normalize();

    model
        .world_from_local
        .as_mat4()
        .transform_point3(local_position)
        .as_dvec3()
}

fn approximate_world_position(
    view_coordinates: &[Coordinate],
    approximations: &[SurfaceApproximation],
    origin_lod: u32,
    second_order: bool,
    view_position: DVec3,
    (tile, tile_uv): (TileCoordinate, Vec2),
) -> DVec3 {
    let ViewCoordinate {
        xy: view_xy,
        uv: view_uv,
    } = ViewCoordinate::new(view_coordinates[tile.side as usize], origin_lod);

    let &SurfaceApproximation {
        c,
        c_du,
        c_dv,
        c_duu,
        c_duv,
        c_dvv,
    } = &approximations[tile.side as usize];

    let Vec2 { x: u, y: v } = ((tile.xy() - view_xy).as_vec2() + tile_uv - view_uv)
        / TileCoordinate::count(tile.lod) as f32;

    let approximate_relative_position = if second_order {
        c + c_du * u + c_dv * v + c_duu * u * u + c_duv * u * v + c_dvv * v * v
    } else {
        c + c_du * u + c_dv * v
    };

    view_position + approximate_relative_position.as_dvec3()
}

fn random_test_position(
    rng: &mut ThreadRng,
    model: &TerrainModel,
    threshold: f64,
    view_position: DVec3,
) -> DVec3 {
    model.position_local_to_world(
        model.position_world_to_local(
            view_position
                + (rng.gen_range(0.0..1.0)
                    * threshold
                    * DVec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                    )
                    .normalize()),
        ),
        0.0,
    )
}

fn random_view_position(rng: &mut ThreadRng, model: &TerrainModel, max_height: f64) -> DVec3 {
    Coordinate::new(
        rng.gen_range(0..6),
        DVec2::new(rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0)),
    )
    .world_position(&model, rng.gen_range(0.0..max_height as f32))
}

fn tile_coordinate_from_world_position(
    world_position: DVec3,
    lod: u32,
    model: &TerrainModel,
) -> (TileCoordinate, Vec2) {
    let coordinate = Coordinate::from_world_position(world_position, &model);
    let uv = coordinate.uv * (1 << lod) as f64;
    let tile_xy = uv.as_uvec2();
    let tile_uv = uv.fract().as_vec2();

    (
        TileCoordinate::new(coordinate.side, lod, tile_xy.x, tile_xy.y),
        tile_uv,
    )
}

#[derive(Default)]
struct ViewError {
    position: Vec3,
    max_error: f64,
}

#[derive(Resource, Default)]
struct Errors {
    view_errors: Vec<ViewError>,
    max_error: f64,
}

fn compute_errors() -> Errors {
    let mut rng = thread_rng();

    let model = TerrainModel::ellipsoid(DVec3::ZERO, 6378137.0, 6356752.314245, 0.0, 0.0);

    let view_samples = 10000;
    let surface_samples = 100;
    let view_lod = 10;
    let threshold = 0.001 * model.scale();

    // The approximation is as good as the f32 computation (2m max error), at distances below 0.005 * RADIUS (30km) around the camera.
    // With a distance below 0.001 * RADIUS (and an origin lod of 10) the maximum approximation error is around 1 cm.

    let mut count = 0;
    let mut taylor1_max: f64 = 0.0;
    let mut taylor1_avg: f64 = 0.0;
    let mut taylor2_max: f64 = 0.0;
    let mut taylor2_avg: f64 = 0.0;
    let mut f32_max: f64 = 0.0;
    let mut f32_avg: f64 = 0.0;
    let mut cast_max: f64 = 0.0;
    let mut cast_avg: f64 = 0.0;

    let mut view_errors = vec![];

    for _ in 0..view_samples {
        let view_position = random_view_position(&mut rng, &model, threshold);
        let view_coordinate = Coordinate::from_world_position(view_position, &model);

        let view_coordinates = (0..6)
            .map(|side| view_coordinate.project_to_side(side, &model))
            .collect_vec();

        let approximations = view_coordinates
            .iter()
            .map(|&view_coordinate| {
                SurfaceApproximation::compute(view_coordinate, view_position, &model)
            })
            .collect_vec();

        let mut max_error: f64 = 0.0;

        for _ in 0..surface_samples {
            let surface_position = random_test_position(&mut rng, &model, threshold, view_position);

            let coordinate =
                tile_coordinate_from_world_position(surface_position, view_lod, &model);

            let taylor1_error = surface_position.distance(approximate_world_position(
                &view_coordinates,
                &approximations,
                view_lod,
                false,
                view_position,
                coordinate,
            ));
            let taylor2_error = surface_position.distance(approximate_world_position(
                &view_coordinates,
                &approximations,
                view_lod,
                true,
                view_position,
                coordinate,
            ));
            let f32_error = surface_position.distance(f32_world_position(coordinate, &model));
            let cast_error = surface_position.distance(surface_position.as_vec3().as_dvec3());

            count += 1;
            taylor1_max = taylor1_max.max(taylor1_error);
            taylor1_avg = taylor1_avg + taylor1_error;
            taylor2_max = taylor2_max.max(taylor2_error);
            taylor2_avg = taylor2_avg + taylor2_error;
            f32_max = f32_max.max(f32_error);
            f32_avg = f32_avg + f32_error;
            cast_max = cast_max.max(cast_error);
            cast_avg = cast_avg + cast_error;

            max_error = max_error.max(taylor2_error);
        }

        view_errors.push(ViewError {
            position: (view_position / model.scale() * RADIUS).as_vec3(), // (view_position.normalize() * RADIUS).as_vec3(),
            max_error,
        });
    }

    taylor1_avg = taylor1_avg / count as f64;
    taylor2_avg = taylor2_avg / count as f64;
    f32_avg = f32_avg / count as f64;
    cast_avg = cast_avg / count as f64;

    println!("With a threshold factor of {} and an view LOD of {view_lod}, the error in a sample distance of {:.4} m around the camera looks like this.", threshold / model.scale(), threshold);
    println!("The world space error introduced by the first order taylor approximation is {:.4} m on average and {:.4} m at the maximum.", taylor1_avg, taylor1_max);
    println!("The world space error introduced by the second order taylor approximation is {:.4} m on average and {:.4} m at the maximum.", taylor2_avg, taylor2_max);
    println!("The world space error introduced by computing the position using f32 is {:.4} m on average and {:.4} m at the maximum.", f32_avg, f32_max);
    println!("The world space error introduced by downcasting from f64 to f32 is {:.4} m on average and {:.4} m at the maximum.", cast_avg, cast_max);

    Errors {
        view_errors,
        max_error: taylor2_max,
    }
}

fn main() {
    let errors = compute_errors();

    if true {
        App::new()
            .add_plugins((
                DefaultPlugins.build().disable::<TransformPlugin>(),
                TerrainPlugin,
                TerrainDebugPlugin,
            ))
            .insert_resource(errors)
            .insert_resource(ClearColor(basic::WHITE.into()))
            .add_systems(Startup, setup)
            .add_systems(Update, update)
            .run();
    }
}

const RADIUS: f64 = 10.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let camera_position = -DVec3::X * RADIUS * 3.0;

    commands.spawn_big_space(ReferenceFrame::default(), |root| {
        let frame = root.frame().clone();

        let mut material = StandardMaterial::from_color(basic::GRAY);
        material.unlit = true;

        root.spawn_spatial(PbrBundle {
            mesh: meshes.add(Sphere::new(0.9999 * RADIUS as f32).mesh().ico(20).unwrap()),
            material: materials.add(material),
            ..default()
        });

        let (camera_cell, camera_translation) = frame.translation_to_grid(camera_position);
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

fn update(errors: Res<Errors>, mut gizmos: Gizmos) {
    let model = TerrainModel::sphere(DVec3::ZERO, RADIUS, 0.0, 0.0);

    draw_earth(&mut gizmos, &model, 3, DVec3::ZERO);

    for view_error in &errors.view_errors {
        let rel_error = (view_error.max_error / errors.max_error) as f32;

        gizmos.sphere(
            view_error.position,
            Quat::IDENTITY,
            0.01 * rel_error * RADIUS as f32,
            Hsva::from(basic::RED).with_saturation(rel_error),
        );
    }
}
