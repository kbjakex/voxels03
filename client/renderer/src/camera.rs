use std::f32::consts::{PI, TAU};

use glam::{Mat4, Vec2, Vec3};

// This is in the renderer crate for two reasons:
// 1. Both renderer and the client need access, and dumping this into a crate
//    shared by both feels off
// 2. More importantly: the camera isn't totally independent of the renderer.
//    It knows, for instance, what type of projection is expected, and the
//    handedness of the graphics API, and global up direction.

pub struct Camera {
    projection: Mat4,
    view: Mat4,
    proj_view: Mat4,

    facing: Vec3,
    right: Vec3,
    yaw_rad: f32,
    pitch_rad: f32,

    pos: Vec3,

    fov_rad: f32,
}

impl Camera {
    /// Creates a Y-up camera. About yaw and pitch:
    /// Pitch of PI/2 is straight up, -PI/2 is straight down.
    /// Yaw of 0 will look at +X; increasing the yaw will rotate right.
    pub fn new(pos: Vec3, (yaw_rad, pitch_rad): (f32, f32), fov_rad: f32, win_size: Vec2) -> Self {
        let facing = euler_to_vec(yaw_rad, pitch_rad);
        let projection = Self::create_projection_matrix(fov_rad, win_size);
        let view = Mat4::look_at_rh(pos, pos + facing, Vec3::Y);
        Camera {
            projection,
            view,
            proj_view: projection * view,
            facing,
            right: compute_right_dir(facing),
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            pos,
            fov_rad,
        }
    }

    /// Meant to be called once per frame, before rendering but after use, to update the cached values.
    pub fn update(&mut self) {
        self.view = Mat4::look_at_rh(self.pos, self.pos + self.facing, Vec3::Y);
        self.proj_view = self.projection * self.view;
    }

    pub fn rotate(&mut self, yaw_delta_rad: f32, pitch_delta_rad: f32) {
        self.set_rotation(self.yaw_rad + yaw_delta_rad, self.pitch_rad - pitch_delta_rad);
    }

    pub fn set_rotation(&mut self, yaw_rad: f32, pitch_rad: f32) {
        self.yaw_rad = yaw_rad % TAU;
        if self.yaw_rad < 0.0 {
            self.yaw_rad += TAU;
        }

        // Prevent looking actually straight up or down: this will cause all sorts of issues with
        // the view matrix
        self.pitch_rad = pitch_rad.clamp(-PI / 2.0 + 0.001, PI / 2.0 - 0.001);

        self.facing = euler_to_vec(self.yaw_rad, self.pitch_rad);
        self.right = compute_right_dir(self.facing);
    }

    pub fn set_fov(&mut self, fov_rad: f32, win_size: Vec2) {
        self.fov_rad = fov_rad;
        self.on_window_resize(win_size);
    }

    pub fn on_window_resize(&mut self, new_size: Vec2) {
        self.projection = Self::create_projection_matrix(self.fov_rad, new_size);
    }

    pub fn move_by(&mut self, velocity: Vec3) {
        self.pos += velocity;
    }

    pub fn move_to(&mut self, pos: Vec3) {
        self.pos = pos;
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    pub fn facing(&self) -> Vec3 {
        self.facing
    }

    pub fn right(&self) -> Vec3 {
        self.right
    }

    pub fn yaw(&self) -> f32 {
        self.yaw_rad
    }

    pub fn pitch(&self) -> f32 {
        self.pitch_rad
    }

    pub fn proj_view_matrix(&self) -> Mat4 {
        self.proj_view
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.projection
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.view
    }

    fn create_projection_matrix(fov_rad: f32, win_size: Vec2) -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(fov_rad, win_size.x / win_size.y, 0.1)
    }
}

pub fn euler_to_vec(yaw_rad: f32, pitch_rad: f32) -> Vec3 {
    let (yaw_sin, yaw_cos) = yaw_rad.sin_cos();
    let (pitch_sin, pitch_cos) = pitch_rad.sin_cos();
    Vec3::new(
        yaw_cos * pitch_cos, 
        pitch_sin, 
        yaw_sin * pitch_cos
    )
}

// For clarity: I can never remember which way around
// the vectors need to be, nor do I want to think about it
fn compute_right_dir(facing: Vec3) -> Vec3 {
    facing.cross(Vec3::Y)
}
