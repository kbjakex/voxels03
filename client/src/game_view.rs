pub mod state;

use glam::{Vec3, Vec2, vec2};
use log::debug;
use netcode::login::LoginResponse;
use renderer::game_renderer::GameRenderer;
use winit::{event::{Event, WindowEvent, ElementState, MouseButton, DeviceEvent}, dpi::LogicalPosition};

use crate::{views::{StateChange, exit}, resources::Resources, world::chunk::WorldBlockPosExt, util::input::Key};

use self::state::GameState;

pub struct GameView {
    state: GameState,
    renderer: GameRenderer,
    focused: bool,
    mouse_motion_accumulator: Vec2,
}

impl GameView {
    pub fn new(login_response: LoginResponse, res: &mut Resources) -> anyhow::Result<Self> {
        let chunk_pos = login_response.position.as_ivec3().to_chunk_pos();
        Ok(Self {
            state: GameState::new(login_response, res),
            renderer: GameRenderer::new(chunk_pos, &mut res.renderer)?,
            focused: false,
            mouse_motion_accumulator: Vec2::ZERO
        })
    }
}

impl GameView {
    pub fn on_enter_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Entering game view");
        Ok(())
    }

    pub fn on_exit_view(&mut self, _res: &mut Resources) -> anyhow::Result<()> {
        debug!("Leaving game view");
        Ok(())
    }

    pub fn on_update(&mut self, res: &mut Resources) -> Option<Box<StateChange>> {
        if res.input.keyboard.just_pressed(Key::Escape) {
            if !self.focused {
                return exit();
            }
            self.focused = false;
            res.window_handle.set_cursor_visible(true);
        }

        if self.focused {
            self.do_player_movement(res);
        }

        self.state.camera.update();
        self.renderer.render(&self.state.camera, &mut res.renderer).unwrap();
        None
    }

    pub fn on_event(&mut self, event: Event<()>, res: &mut Resources) -> Option<Box<StateChange>> {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(..) = event {
                self.renderer.handle_window_resize(&res.renderer);
            }
            else if let WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } = event {
                if !self.focused {
                    self.focused = true;
                    //res.window_handle.set_cursor_visible(false);
                    _ = res.window_handle.set_cursor_position(LogicalPosition::<f32>::from((res.window_size.w_h_f32 * 0.5).to_array()));
                }
            }
            else if let WindowEvent::CursorMoved { .. } = event {
                if self.focused {
                    _ = res.window_handle.set_cursor_position(LogicalPosition::<f32>::from((res.window_size.w_h_f32 * 0.5).to_array()));
                }
            }
        }
        else if let Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta: (x, y) }, ..} = event {
            self.mouse_motion_accumulator += vec2(x as f32, y as f32);
        }

        None
    }

    fn do_player_movement(&mut self, res: &mut Resources) {
        let keyboard = &mut res.input.keyboard;
        
        let right = keyboard.get_axis(Key::D, Key::A);
        let up = keyboard.get_axis(Key::Space, Key::LShift);
        let fwd = keyboard.get_axis(Key::W, Key::S);
        
        if right != 0 || up != 0 || fwd != 0 {
            let (ys, yc) = self.state.camera.yaw().sin_cos();
            let fwd_dir = Vec3::new(yc, 0.0, ys);
            let up_dir = Vec3::Y;
            let right_dir = fwd_dir.cross(up_dir);
            
            let hor_acc = (right as f32 * right_dir + fwd as f32 * fwd_dir).normalize_or_zero();
            let acc = (hor_acc + up as f32 * up_dir) * 1.0;
            
            self.state.camera.move_by(acc * res.time.dt_secs);
        }

        let delta = std::mem::replace(&mut self.mouse_motion_accumulator, Vec2::ZERO) * 0.0025;
        debug!("{delta}");
        self.state.camera.rotate(delta.x, delta.y);
    }
}