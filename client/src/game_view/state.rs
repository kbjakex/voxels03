use netcode::login::LoginResponse;
use renderer::camera::Camera;

use crate::resources::Resources;


pub struct GameState {
    pub camera: Camera
}

impl GameState {
    pub fn new(login_response: LoginResponse, res: &mut Resources) -> Self {
        Self {
            camera: Camera::new(login_response.position, (0.0, 0.0), f32::to_radians(80.0), res.window_size.w_h_f32)
        }
    }
}