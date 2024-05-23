use bevy::math::{DMat3, DVec2, DVec3, IVec2, Vec2, Vec3};

const C_SQR: f64 = 0.87 * 0.87;
pub(crate) const THRESHOLD_LOD: i32 = 10; // the LOD, at which we swap to the relative position approximation
const MARGIN: i32 = 8; // defines the approximation area around the camera for tiles with threshold LOD
pub(crate) const SCALE: f64 = MARGIN as f64 / (1 << THRESHOLD_LOD) as f64; // the coefficient when translating from relative to absolute uv coordinates

pub(crate) fn cube_to_sphere(uv: DVec2) -> DVec2 {
    let w = uv * ((1.0 + C_SQR) / (1.0 + C_SQR * uv * uv)).powf(0.5);
    0.5 * w + 0.5
}

pub(crate) fn sphere_to_cube(st: DVec2) -> DVec2 {
    let w = (st - 0.5) / 0.5;
    w / (1.0 + C_SQR - C_SQR * w * w).powf(0.5)
}

pub(crate) fn world_position(side: u32, uv: DVec2) -> DVec3 {
    match side {
        0 => DVec3::new(-1.0, -uv.y, uv.x),
        1 => DVec3::new(uv.x, -uv.y, 1.0),
        2 => DVec3::new(uv.x, 1.0, uv.y),
        3 => DVec3::new(1.0, -uv.x, uv.y),
        4 => DVec3::new(uv.y, -uv.x, -1.0),
        5 => DVec3::new(uv.y, -1.0, uv.x),
        _ => unreachable!(),
    }
    .normalize()
}

#[derive(Clone, Copy)]
enum SideInfo {
    Fixed0,
    Fixed1,
    PositiveS,
    PositiveT,
}

impl SideInfo {
    const EVEN_LIST: [[SideInfo; 2]; 6] = [
        [SideInfo::PositiveS, SideInfo::PositiveT],
        [SideInfo::Fixed0, SideInfo::PositiveT],
        [SideInfo::Fixed0, SideInfo::PositiveS],
        [SideInfo::PositiveT, SideInfo::PositiveS],
        [SideInfo::PositiveT, SideInfo::Fixed0],
        [SideInfo::PositiveS, SideInfo::Fixed0],
    ];
    const ODD_LIST: [[SideInfo; 2]; 6] = [
        [SideInfo::PositiveS, SideInfo::PositiveT],
        [SideInfo::PositiveS, SideInfo::Fixed1],
        [SideInfo::PositiveT, SideInfo::Fixed1],
        [SideInfo::PositiveT, SideInfo::PositiveS],
        [SideInfo::Fixed1, SideInfo::PositiveS],
        [SideInfo::Fixed1, SideInfo::PositiveT],
    ];

    fn project_to_side(side: u32, other_side: u32) -> [SideInfo; 2] {
        let index = ((6 + other_side - side) % 6) as usize;

        if side % 2 == 0 {
            SideInfo::EVEN_LIST[index]
        } else {
            SideInfo::ODD_LIST[index]
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Coordinate {
    pub(crate) side: u32,
    pub(crate) st: DVec2,
}

impl Coordinate {
    pub(crate) fn from_world_position(world_position: DVec3) -> Self {
        let normal = world_position.normalize();
        let abs_normal = normal.abs();

        let (side, uv) = if abs_normal.x > abs_normal.y && abs_normal.x > abs_normal.z {
            if normal.x < 0.0 {
                (0, DVec2::new(-normal.z / normal.x, normal.y / normal.x))
            } else {
                (3, DVec2::new(-normal.y / normal.x, normal.z / normal.x))
            }
        } else if abs_normal.z > abs_normal.y {
            if normal.z > 0.0 {
                (1, DVec2::new(normal.x / normal.z, -normal.y / normal.z))
            } else {
                (4, DVec2::new(normal.y / normal.z, -normal.x / normal.z))
            }
        } else {
            if normal.y > 0.0 {
                (2, DVec2::new(normal.x / normal.y, normal.z / normal.y))
            } else {
                (5, DVec2::new(-normal.z / normal.y, -normal.x / normal.y))
            }
        };

        let st = cube_to_sphere(uv);

        Self { side, st }
    }

    pub(crate) fn project_to_side(self, side: u32) -> Self {
        let info = SideInfo::project_to_side(self.side, side);

        let st = info
            .map(|info| match info {
                SideInfo::Fixed0 => 0.0,
                SideInfo::Fixed1 => 1.0,
                SideInfo::PositiveS => self.st.x,
                SideInfo::PositiveT => self.st.y,
            })
            .into();

        Self { side, st }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct Tile {
    pub(crate) xy: IVec2,
    pub(crate) lod: i32,
    pub(crate) side: u32,
}

impl Tile {
    pub(crate) fn new(side: u32, lod: i32, x: i32, y: i32) -> Self {
        Self {
            xy: IVec2::new(x, y),
            lod,
            side,
        }
    }

    fn origin_coordinate(coordinate: Coordinate) -> Coordinate {
        let st = (coordinate.st * Self::tile_count(THRESHOLD_LOD) as f64).round()
            / Tile::tile_count(THRESHOLD_LOD) as f64;

        Coordinate { st, ..coordinate }
    }

    pub(crate) fn tile_count(lod: i32) -> i32 {
        1 << lod
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CameraParams {
    pub(crate) world_position: DVec3,
    pub(crate) origin_coordinates: [Coordinate; 6],
}

impl CameraParams {
    pub(crate) fn new(world_position: DVec3) -> Self {
        let coordinate = Coordinate::from_world_position(world_position);
        let origin_coordinate = Tile::origin_coordinate(coordinate);

        let mut origin_coordinates = [Coordinate::default(); 6];

        for side in 0..6 {
            origin_coordinates[side as usize] = origin_coordinate.project_to_side(side);
        }

        Self {
            world_position,
            origin_coordinates,
        }
    }

    pub(crate) fn relative_st(&self, tile: Tile, vertex_offset: Vec2) -> Vec2 {
        ((tile.xy.as_dvec2() + vertex_offset.as_dvec2()
            - self.origin_coordinates[tile.side as usize].st * Tile::tile_count(tile.lod) as f64)
            / MARGIN as f64)
            .as_vec2()
    }

    pub(crate) fn relative_position(&self, relative_st: Vec2, side: u32) -> DVec3 {
        let st = self.origin_coordinates[side as usize].st + relative_st.as_dvec2() * SCALE;
        let uv = sphere_to_cube(st);

        world_position(side, uv) - self.world_position
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct SideCoefficients {
    pub(crate) delta_relative_st: Vec2,
    pub(crate) origin_xy: IVec2,
    pub(crate) c: Vec3,
    pub(crate) c_s: Vec3,
    pub(crate) c_t: Vec3,
    pub(crate) c_ss: Vec3,
    pub(crate) c_st: Vec3,
    pub(crate) c_tt: Vec3,
}

#[derive(Clone, Debug)]
pub(crate) struct CameraApproximationParams {
    pub(crate) sides: [SideCoefficients; 6],
}

impl CameraApproximationParams {
    pub(crate) fn compute(world_position: DVec3) -> CameraApproximationParams {
        let camera_coordinate = Coordinate::from_world_position(world_position);
        let origin_coordinate = Tile::origin_coordinate(camera_coordinate);

        // We want to approximate the Camera::relative_position method using a second order taylor series.
        // For that, we have to calculate the Taylor coefficients for each cube side separately.
        // As the basis, we use the projected origin st coordinates to the specific side.
        // Then we calculate the relative position vector and derivatives at the relative coordinate (0,0).

        // use WolframAlpha to get first and second order derivatives
        // f(x)=(2*(x*g+o)-1)/sqrt(1+c-c*(2*(x*g+o)-1)^2)
        // l(s,t)=sqrt(1+u(s)^2+v(t)^2)
        // a(s,t)=1/l(s,t)
        // b(s,t)=u(s)/l(s,t)
        // c(s,t)=v(t)/l(s,t)

        // f'(x)  =(2(c+1)g)/(1-4c(gx+o-1)(gx+o))^(3/2)
        // f''(x) =(12c(c+1)g^2(2gx+2o-1))/(1-4c(gx+o-1)(gx+o))^(5/2)
        // f'''(x)=(24c(c+1)g^3(c(16o(2gx-1)+16gx(gx-1)+16o^2+5)+1))/(1-4c(gx+o-1)(gx+o))^(7/2)

        // Transforms relative st coordinates (x) around the origin st coordinate (o) to uv coordinates
        fn f(x: f64, o: f64) -> f64 {
            (2.0 * (x * SCALE + o) - 1.0)
                / (1.0 + C_SQR - C_SQR * (2.0 * (x * SCALE + o) - 1.0).powi(2)).powf(0.5)
        }

        fn f_dx(x: f64, o: f64) -> f64 {
            2.0 * (C_SQR + 1.0) * SCALE
                / (1.0 - 4.0 * C_SQR * (SCALE * x + o - 1.0) * (SCALE * x + o)).powf(1.5)
        }

        fn f_dxx(x: f64, o: f64) -> f64 {
            12.0 * C_SQR * (C_SQR + 1.0) * SCALE * SCALE * (2.0 * SCALE * x + 2.0 * o - 1.0)
                / (1.0 - 4.0 * C_SQR * (SCALE * x + o - 1.0) * (SCALE * x + o)).powf(2.5)
        }

        let mut sides = [SideCoefficients::default(); 6];

        for side in 0..6 {
            let origin_coordinate = origin_coordinate.project_to_side(side);
            let camera_coordinate = camera_coordinate.project_to_side(side);
            let origin_xy =
                (origin_coordinate.st * Tile::tile_count(THRESHOLD_LOD) as f64).as_ivec2();
            let delta_relative_st =
                ((origin_coordinate.st - camera_coordinate.st) / SCALE).as_vec2();

            let DVec2 { x: s, y: t } = camera_coordinate.st;
            let (ds, dt) = (0.0, 0.0); // we are interested in the coefficients at the relative coordinates st = (0, 0)

            let u = f(ds, s);
            let u_ds = f_dx(ds, s);
            let u_dss = f_dxx(ds, s);

            let v = f(dt, t);
            let v_dt = f_dx(dt, t);
            let v_dtt = f_dxx(dt, t);

            let l = (1.0 + u * u + v * v).sqrt();
            let l_ds = u * u_ds / l;
            let l_dt = v * v_dt / l;
            let l_dss = (u * u_dss * l * l + (v * v + 1.0) * u_ds * u_ds) / l.powf(3.0);
            let l_dst = -(u * v * u_ds * v_dt) / l.powf(3.0);
            let l_dtt = (v * v_dtt * l * l + (u * u + 1.0) * v_dt * v_dt) / l.powf(3.0);

            let a = 1.0;
            let a_ds = -l_ds;
            let a_dt = -l_dt;
            let a_dss = 2.0 * l_ds * l_ds - l * l_dss;
            let a_dst = 2.0 * l_ds * l_dt - l * l_dst;
            let a_dtt = 2.0 * l_dt * l_dt - l * l_dtt;

            let b = u;
            let b_ds = -u * l_ds + l * u_ds;
            let b_dt = -u * l_dt;
            let b_dss = 2.0 * u * l_ds * l_ds - l * (2.0 * u_ds * l_ds + u * l_dss) + u_dss * l * l;
            let b_dst = 2.0 * u * l_ds * l_dt - l * (u_ds * l_dt + u * l_dst);
            let b_dtt = 2.0 * u * l_dt * l_dt - l * u * l_dtt;

            let c = v;
            let c_ds = -v * l_ds;
            let c_dt = -v * l_dt + l * v_dt;
            let c_dss = 2.0 * v * l_ds * l_ds - l * v * l_dss;
            let c_dst = 2.0 * v * l_ds * l_dt - l * (v_dt * l_ds + v * l_dst);
            let c_dtt = 2.0 * v * l_dt * l_dt - l * (2.0 * v_dt * l_dt + v * l_dtt) + v_dtt * l * l;

            let v = DVec3::new(a, b, c) / l;
            let v_ds = DVec3::new(a_ds, b_ds, c_ds) / (l * l);
            let v_dt = DVec3::new(a_dt, b_dt, c_dt) / (l * l);
            let v_dss = DVec3::new(a_dss, b_dss, c_dss) / (l * l * l);
            let v_dst = DVec3::new(a_dst, b_dst, c_dst) / (l * l * l);
            let v_dtt = DVec3::new(a_dtt, b_dtt, c_dtt) / (l * l * l);

            let s = match side {
                0 => DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0]),
                1 => DMat3::from_cols_array(&[0.0, 1.0, 0.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0]),
                2 => DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
                3 => DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
                4 => DMat3::from_cols_array(&[0.0, 0.0, 1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 0.0]),
                5 => DMat3::from_cols_array(&[0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0]),
                _ => unreachable!(),
            }
            .transpose();

            sides[side as usize] = SideCoefficients {
                delta_relative_st,
                origin_xy,
                c: (s * v - world_position).as_vec3(),
                c_s: (s * v_ds).as_vec3(),
                c_t: (s * v_dt).as_vec3(),
                c_ss: (s * v_dss / 2.0).as_vec3(),
                c_st: (s * v_dst).as_vec3(),
                c_tt: (s * v_dtt / 2.0).as_vec3(),
            };
        }

        CameraApproximationParams { sides }
    }

    pub(crate) fn relative_st(&self, tile: Tile, vertex_offset: Vec2) -> Vec2 {
        let lod_difference = tile.lod - THRESHOLD_LOD;
        let tile_offset = tile.xy - self.sides[tile.side as usize].origin_xy << lod_difference;
        let margin = MARGIN << lod_difference;

        (tile_offset.as_vec2() + vertex_offset) / margin as f32
    }

    pub(crate) fn relative_position(&self, relative_st: Vec2, side: u32) -> Vec3 {
        let &SideCoefficients {
            delta_relative_st,
            c,
            c_s,
            c_t,
            c_ss,
            c_st,
            c_tt,
            ..
        } = &self.sides[side as usize];
        let Vec2 { x: s, y: t } = relative_st + delta_relative_st;

        c + c_s * s + c_t * t + c_ss * s * s + c_st * s * t + c_tt * t * t
    }
}
