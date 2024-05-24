use bevy::math::{DMat3, DVec2, DVec3, IVec2, Vec2, Vec3};

/// The square of the parameter c of the algebraic sigmoid function, used to convert between uv and st coordinates.
const C_SQR: f64 = 0.87 * 0.87;

const SIDE_MATRICES: [DMat3; 6] = [
    DMat3::from_cols_array(&[-1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, -1.0, 0.0]),
    DMat3::from_cols_array(&[0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0]),
    DMat3::from_cols_array(&[0.0, 0.0, -1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0]),
    DMat3::from_cols_array(&[0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]),
];

/// Converts uv coordinates in range [-1,1] to st coordinates in range [0,1].
/// The uv coordinates are spaced equally on the surface of the cube and
/// the st coordinates are spaced equally on the surface of the sphere.
fn cube_to_sphere(uv: DVec2) -> DVec2 {
    let w = uv * ((1.0 + C_SQR) / (1.0 + C_SQR * uv * uv)).powf(0.5);
    0.5 * w + 0.5
}

/// Converts st coordinates in range [0,1] to uv coordinates in range [-1,1].
/// The uv coordinates are spaced equally on the surface of the cube and
/// the st coordinates are spaced equally on the surface of the sphere.
fn sphere_to_cube(st: DVec2) -> DVec2 {
    let w = (st - 0.5) / 0.5;
    w / (1.0 + C_SQR - C_SQR * w * w).powf(0.5)
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

/// Describes a location on the unit cube sphere.
/// The side index refers to one of the six cube faces and the st coordinate describes the location within this side.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Coordinate {
    pub(crate) side: u32,
    pub(crate) st: DVec2,
}

impl Coordinate {
    pub(crate) fn new(side: u32, st: DVec2) -> Self {
        Self { side, st }
    }

    pub(crate) fn from_local_position(world_position: DVec3) -> Self {
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

    /// Calculates the local unit sphere position of this coordinate.
    pub(crate) fn to_local_position(self) -> DVec3 {
        let uv = sphere_to_cube(self.st);

        match self.side {
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

    /// Projects the coordinate onto one of the six cube faces.
    /// Thereby it chooses the closest location on this face to the original coordinate.
    fn project_to_side(self, side: u32) -> Self {
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

/// Describes a quadtree tile of the cube sphere.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Tile {
    /// The tile index (at the LOD of this tile) from the top-left corner of the cube face.
    pub(crate) xy: IVec2,
    /// The lod of the tile.
    pub(crate) lod: i32,
    /// Describes which side of the cube sphere the tile lies within.
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

    pub(crate) fn tile_count(lod: i32) -> i32 {
        1 << lod
    }
}

/// Parameters of the camera used to compute the position of a location on the sphere's surface relative to the camera.
/// This can be calculated directly using f64 operations, or approximated using a Taylor series and f32 operations.
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct SideParameter {
    /// The tile index of the origin tile projected to this side.
    pub(crate) origin_xy: IVec2,
    /// The st coordinate of the origin tile projected to this side.
    pub(crate) origin_st: DVec2,
    /// The offset between the camera st coordinate and the origin st coordinate in relative st space.
    /// This can be used to translate from st coordinates relative to the origin tile to st coordinates relative to the camera coordinate in the shader.
    pub(crate) delta_relative_st: Vec2,
    /// The constant coefficient of the series.
    /// Describes the offset between the location vertically under camera and the camera position.
    pub(crate) c: Vec3,
    /// The linear coefficient of the series with respect to s.
    pub(crate) c_s: Vec3,
    /// The linear coefficient of the series with respect to t.
    pub(crate) c_t: Vec3,
    /// The quadratic coefficient of the series with respect to s and s.
    /// This value is pre-multiplied with 0.5.
    pub(crate) c_ss: Vec3,
    /// The quadratic coefficient of the series with respect to s and t.
    pub(crate) c_st: Vec3,
    /// The quadratic coefficient of the series with respect to t and t.
    /// This value is pre-multiplied with 0.5.
    pub(crate) c_tt: Vec3,
}

#[derive(Clone, Debug)]
pub(crate) struct CameraParameter {
    /// The world position of the camera.
    pub(crate) position: DVec3,
    pub(crate) coordinate: Coordinate,
    /// The LOD, at which we swap to the relative position approximation.
    /// The tile under the camera (with the THRESHOLD_LOD) is the origin for the Taylor series.
    pub(crate) origin_lod: i32,
    pub(crate) earth: Earth,
    /// The parameters of the six cube sphere faces.
    pub(crate) sides: [SideParameter; 6],
}

impl CameraParameter {
    /// Computes the camera parameters based on the camera position.
    pub(crate) fn compute(
        camera_position: DVec3,
        earth: Earth,
        origin_lod: i32,
    ) -> CameraParameter {
        let local_position = earth.world_to_local(camera_position);

        // Coordinate of the location vertically below the camera.
        let camera_coordinate = Coordinate::from_local_position(local_position);
        // Coordinate of the tile closest to the camera coordinate.
        let origin_coordinate = Self::origin_coordinate(camera_coordinate, origin_lod);

        // We want to approximate the position relative to the camera using a second order Taylor series.
        // For that, we have to calculate the Taylor coefficients for each cube side separately.
        // As the basis, we use the camera coordinate projected to the specific side.
        // Then we calculate the relative position vector and derivatives at the camera coordinate.

        // u(s)=(2s-1)/sqrt(1-4cs(s-1))
        // v(t)=(2t-1)/sqrt(1-4ct(t-1))
        // l(s,t)=sqrt(1+u(s)^2+v(t)^2)
        // a(s,t)=1/l(s,t)
        // b(s,t)=u(s)/l(s,t)
        // c(s,t)=v(t)/l(s,t)

        let mut sides = [SideParameter::default(); 6];

        for (side, &side_matrix) in SIDE_MATRICES.iter().enumerate() {
            let origin_coordinate = origin_coordinate.project_to_side(side as u32);
            let camera_coordinate = camera_coordinate.project_to_side(side as u32);
            let origin_st = origin_coordinate.st;
            let origin_xy = (origin_coordinate.st * Tile::tile_count(origin_lod) as f64).as_ivec2();
            let delta_relative_st = (origin_coordinate.st - camera_coordinate.st).as_vec2();

            let r = earth.radius;
            let DVec2 { x: s, y: t } = camera_coordinate.st;

            let u_denom = (1.0 - 4.0 * C_SQR * s * (s - 1.0)).sqrt();
            let u = (2.0 * s - 1.0) / u_denom;
            let u_ds = 2.0 * (C_SQR + 1.0) / u_denom.powi(3);
            let u_dss = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * s - 1.0) / u_denom.powi(5);

            let v_denom = (1.0 - 4.0 * C_SQR * t * (t - 1.0)).sqrt();
            let v = (2.0 * t - 1.0) / v_denom;
            let v_dt = 2.0 * (C_SQR + 1.0) / v_denom.powi(3);
            let v_dtt = 12.0 * C_SQR * (C_SQR + 1.0) * (2.0 * t - 1.0) / v_denom.powi(5);

            let l = (1.0 + u * u + v * v).sqrt();
            let l_ds = u * u_ds / l;
            let l_dt = v * v_dt / l;
            let l_dss = (u * u_dss * l * l + (v * v + 1.0) * u_ds * u_ds) / l.powi(3);
            let l_dst = -(u * v * u_ds * v_dt) / l.powi(3);
            let l_dtt = (v * v_dtt * l * l + (u * u + 1.0) * v_dt * v_dt) / l.powi(3);

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

            let p = r * side_matrix * DVec3::new(a, b, c) / l;
            let p_ds = r * side_matrix * DVec3::new(a_ds, b_ds, c_ds) / l.powi(2);
            let p_dt = r * side_matrix * DVec3::new(a_dt, b_dt, c_dt) / l.powi(2);
            let p_dss = r * side_matrix * DVec3::new(a_dss, b_dss, c_dss) / l.powi(3);
            let p_dst = r * side_matrix * DVec3::new(a_dst, b_dst, c_dst) / l.powi(3);
            let p_dtt = r * side_matrix * DVec3::new(a_dtt, b_dtt, c_dtt) / l.powi(3);

            sides[side] = SideParameter {
                origin_xy,
                origin_st,
                delta_relative_st,
                c: (p + earth.position - camera_position).as_vec3(),
                c_s: p_ds.as_vec3(),
                c_t: p_dt.as_vec3(),
                c_ss: (p_dss / 2.0).as_vec3(),
                c_st: p_dst.as_vec3(),
                c_tt: (p_dtt / 2.0).as_vec3(),
            };
        }

        CameraParameter {
            earth,
            origin_lod,
            coordinate: camera_coordinate,
            position: camera_position,
            sides,
        }
    }

    /// Computes the camera origin tile based on the camera's coordinate.
    /// This is a tile with the threshold lod that is closest to the camera.
    fn origin_coordinate(coordinate: Coordinate, origin_lod: i32) -> Coordinate {
        let tile_count = Tile::tile_count(origin_lod) as f64;
        let st = (coordinate.st * tile_count).round() / tile_count;

        Coordinate { st, ..coordinate }
    }

    /// Computes the relative st coordinate of a vertex inside the tile.
    /// This is the difference between the st coordinate of the vertex, and that of the location vertically below the camera.
    /// This difference is divided by the SCALE factor, to be bound inside the margin.
    pub(crate) fn relative_st(&self, tile: Tile, vertex_offset: Vec2) -> Vec2 {
        (tile.xy.as_dvec2() + vertex_offset.as_dvec2()
            - self.sides[tile.side as usize].origin_st * Tile::tile_count(tile.lod) as f64)
            .as_vec2()
            / (1 << self.origin_lod) as f32
    }

    /// Based on the relative st coordinate, calculates the relative position of the location to the camera.
    pub(crate) fn relative_position(&self, relative_st: Vec2, side: u32) -> DVec3 {
        let st = self.sides[side as usize].origin_st + relative_st.as_dvec2();
        let local_position = Coordinate::new(side, st).to_local_position();

        self.earth.local_to_world(local_position) - self.position
    }

    /// Approximates the relative st coordinate of a vertex inside the tile.
    /// By computing the tile offset between this tile and the origin tile with integer math,
    /// high precision relative st can be guaranteed even for high LODs.
    pub(crate) fn approximate_relative_st(&self, tile: Tile, vertex_offset: Vec2) -> Vec2 {
        let lod_difference = tile.lod - self.origin_lod;
        let tile_offset = tile.xy - self.sides[tile.side as usize].origin_xy << lod_difference;

        (tile_offset.as_vec2() + vertex_offset) / (1 << tile.lod) as f32
    }

    /// Based on the relative st coordinate, approximates the relative position of the location to the camera.
    /// Uses the precomputed Taylor coefficients to provide a high precision approximation of the relative position in close proximity to the camera.
    pub(crate) fn approximate_relative_position(&self, relative_st: Vec2, side: u32) -> Vec3 {
        let &SideParameter {
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

#[derive(Copy, Clone, Debug)]
pub(crate) struct Earth {
    pub(crate) position: DVec3,
    pub(crate) radius: f64,
}

impl Earth {
    pub(crate) fn local_to_world(&self, local_position: DVec3) -> DVec3 {
        self.position + local_position * self.radius
    }

    pub(crate) fn world_to_local(&self, world_position: DVec3) -> DVec3 {
        (world_position - self.position) / self.radius
    }
}
