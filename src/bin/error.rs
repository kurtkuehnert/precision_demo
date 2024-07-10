use bevy::{
    color::palettes::basic,
    math::{DVec2, DVec3},
    prelude::*,
};
use precision_demo::{
    big_space::{BigSpaceCommands, BigSpacePlugin, ReferenceFrame},
    camera::{DebugCameraBundle, DebugCameraController, DebugPlugin},
    draw::draw_earth,
    math::{tile_count, Coordinate, TerrainModel, TerrainModelApproximation, Tile},
};
use rand::{prelude::ThreadRng, thread_rng, Rng};

const C_SQR: f32 = 0.87 * 0.87;

fn f32_world_position(tile: Tile, offset: Vec2, model: &TerrainModel) -> DVec3 {
    let st = (tile.xy.as_vec2() + offset) / tile_count(tile.lod) as f32;

    let w = (st - 0.5) / 0.5;
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

    return model
        .world_from_local
        .as_mat4()
        .transform_point3(local_position)
        .as_dvec3();
}

fn approximate_world_position(
    tile: Tile,
    offset: Vec2,
    approximation: &TerrainModelApproximation,
) -> DVec3 {
    let approximate_relative_st = approximation.approximate_relative_st(tile, offset);
    let approximate_relative_position =
        approximation.approximate_relative_position(approximate_relative_st, tile.side);

    approximation.view_position + approximate_relative_position.as_dvec3()
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
    )
}

fn random_view_position(rng: &mut ThreadRng, model: &TerrainModel, max_height: f64) -> DVec3 {
    Coordinate::new(
        rng.gen_range(0..6),
        DVec2::new(rng.gen_range(0.0..1.0), rng.gen_range(0.0..1.0)),
    )
    .world_position(&model, rng.gen_range(0.0..max_height))
}

#[derive(Copy, Clone, Default)]
struct Error {
    position: Vec3,
    error: f64,
}

impl Error {
    fn max(self, other: Self) -> Self {
        if self.error > other.error {
            self
        } else {
            other
        }
    }
}

#[derive(Default)]
struct ViewErrors {
    position: Vec3,
    errors: Vec<Error>,
    max_error: Error,
}

#[derive(Resource, Default)]
struct Errors {
    view_errors: Vec<ViewErrors>,
    threshold: f32,
    max_error: f64,
}

fn compute_errors() -> Errors {
    let mut rng = thread_rng();

    let model = TerrainModel::new(DVec3::new(0.0, 0.0, 0.0), 6378137.0, 6356752.314245);

    let view_samples = 10000;
    let surface_samples = 1000;
    let origin_lod = 10;
    let threshold = 0.001 * model.scale();

    // The approximation is as good as the f32 computation (2m max error), at distances below 0.005 * RADIUS (30km) around the camera.
    // With a distance below 0.001 * RADIUS (and an origin lod of 10) the maximum approximation error is around 1 cm.

    let mut count = 0;
    let mut approximate_max: f64 = 0.0;
    let mut approximate_avg: f64 = 0.0;
    let mut f32_max: f64 = 0.0;
    let mut f32_avg: f64 = 0.0;
    let mut cast_max: f64 = 0.0;
    let mut cast_avg: f64 = 0.0;

    let mut view_errors = vec![];

    for _ in 0..view_samples {
        let view_position = random_view_position(&mut rng, &model, threshold);
        let approximation =
            TerrainModelApproximation::compute(model.clone(), view_position, origin_lod);

        let mut errors = vec![];

        let mut max_error = Error {
            position: Default::default(),
            error: 0.0,
        };

        for _ in 0..surface_samples {
            let surface_position = random_test_position(&mut rng, &model, threshold, view_position);

            let (tile, offset) = Tile::from_world_position(surface_position, origin_lod, &model);

            let approximate_error =
                surface_position.distance(approximate_world_position(tile, offset, &approximation));
            let f32_error = surface_position.distance(f32_world_position(tile, offset, &model));
            let cast_error = surface_position.distance(surface_position.as_vec3().as_dvec3());

            count += 1;
            approximate_max = approximate_max.max(approximate_error);
            approximate_avg = approximate_avg + approximate_error;
            f32_max = f32_max.max(f32_error);
            f32_avg = f32_avg + f32_error;
            cast_max = cast_max.max(cast_error);
            cast_avg = cast_avg + cast_error;

            let error = Error {
                position: (surface_position.normalize() * RADIUS).as_vec3(),
                error: approximate_error,
            };

            max_error = max_error.max(error);

            errors.push(error);
        }

        view_errors.push(ViewErrors {
            position: (view_position / model.scale() * RADIUS).as_vec3(), // (view_position.normalize() * RADIUS).as_vec3(),
            errors,
            max_error,
        });
    }

    approximate_avg = approximate_avg / count as f64;
    f32_avg = f32_avg / count as f64;
    cast_avg = cast_avg / count as f64;

    println!("The world space error introduced by the taylor approximation is {approximate_avg} on average and {approximate_max} at the maximum.");
    println!("The world space error introduced by computing the position using f32 is {f32_avg} on average and {f32_max} at the maximum.");
    println!("The world space error introduced by downcasting from f64 to f32 is {cast_avg} on average and {cast_max} at the maximum.");

    let errors = Errors {
        view_errors,
        threshold: (threshold / model.scale() * RADIUS) as f32,
        max_error: approximate_max,
    };
    errors
}

fn main() {
    let errors = compute_errors();

    if true {
        App::new()
            .add_plugins((
                DefaultPlugins.build().disable::<TransformPlugin>(),
                BigSpacePlugin::default(),
                DebugPlugin,
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
    let model = TerrainModel::new(DVec3::ZERO, RADIUS, RADIUS);

    draw_earth(&mut gizmos, &model, 3, DVec3::ZERO);

    for view_error in &errors.view_errors {
        // gizmos.sphere(
        //     view_error.position,
        //     Quat::IDENTITY,
        //     0.002 * RADIUS as f32,
        //     basic::GREEN,
        // );

        // gizmos.sphere(
        //     view_error.position,
        //     Quat::IDENTITY,
        //     errors.threshold,
        //     basic::BLUE,
        //  );

        let rel_error = (view_error.max_error.error / errors.max_error) as f32;
        let color = Hsva::from(basic::RED).with_saturation(rel_error);
        gizmos.sphere(
            view_error.max_error.position,
            Quat::IDENTITY,
            0.01 * rel_error * RADIUS as f32,
            color,
        );

        // for error in &view_error.errors {
        //     let rel_error = (error.error / errors.max_error) as f32;
        //
        //     let color = Hsva::from(basic::RED).with_saturation(rel_error);
        //
        //     gizmos.sphere(error.position, Quat::IDENTITY, 0.001 * RADIUS as f32, color);
        // }
    }
}
