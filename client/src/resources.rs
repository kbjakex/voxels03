use std::time::Instant;

use glam::{ivec2, vec2};
use rayon::{ThreadPool, ThreadPoolBuilder};
use renderer::Renderer;
use winit::{event_loop::EventLoop, dpi::LogicalPosition, window::WindowBuilder, event::{Event, WindowEvent}};

use crate::util::{self, input::{Keyboard, Mouse}};


/// Resources that persist during the entire runtime of the
/// game, and are shared across views. Any view-specific state
/// should be elsewhere
pub struct Resources {
    pub time: core::Time,
    pub window_handle: winit::window::Window,
    pub window_size: core::WindowSize,

    pub renderer: Renderer,
    pub input: input::Resources,
    pub thread_pool: ThreadPool,
    pub metrics: metrics::Resources,
}

pub mod core {
    pub struct Time {
        pub at_launch: std::time::Instant, // never updated, measured just before game loop
        pub now: std::time::Instant,       // updated at the very start of each frame
        pub ms_u32: u32,
        pub secs_f32: f32,
        pub dt_secs: f32,
    }

    pub struct WindowSize {
        pub w_h: glam::IVec2,
        pub w_h_f32: glam::Vec2, // convenience
        pub monitor_size_px: winit::dpi::LogicalSize<i32>,
    }
}

pub mod metrics {
    pub struct FrameTime {
        pub avg_fps: f32,
        pub avg_frametime_ms: f32,
        pub frametime_history: [f32; 32],
        pub last_updated: std::time::Instant,
    }

    pub struct Resources {
        pub frame_count: u32,
        pub frame_time: FrameTime,
    }
}

pub mod input {
    use winit::event::ModifiersState;

    use crate::util::input;

    pub struct Resources {
        pub mouse: input::Mouse,
        pub keyboard: input::Keyboard,
        pub settings: input::settings::InputSettings,
        pub clipboard: arboard::Clipboard,

        // tracking for event-based input handling
        pub keyboard_mods: ModifiersState,
    }
}

pub fn init_resources(title: &'static str, event_loop: &EventLoop<()>) -> Resources {
    let now = Instant::now();

    let monitor = event_loop.primary_monitor().unwrap();
    let fullscreen_size = monitor.size().to_logical(monitor.scale_factor());

    let window_size = fullscreen_size;
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(window_size)
        .with_min_inner_size(window_size)
        .with_position(LogicalPosition::new(
            fullscreen_size.width / 2 - window_size.width / 2,
            fullscreen_size.height / 2 - window_size.height / 2,
        ))
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    // Allocate all but one core/thread to the threadpool
    let thread_pool_threads = std::thread::available_parallelism().unwrap().get() - 1;

    Resources {
        time: core::Time {
            at_launch: now,
            now,
            ms_u32: 0,
            secs_f32: 0.0,
            dt_secs: 0.0,
        },
        window_handle: window,
        window_size: core::WindowSize {
            w_h: ivec2(window_size.width, window_size.height),
            w_h_f32: vec2(window_size.width as f32, window_size.height as f32),
            monitor_size_px: fullscreen_size,
        },
        renderer: Renderer::new(),
        input: util::input::init((window_size.width, window_size.height)).unwrap(),
        thread_pool: ThreadPoolBuilder::new()
            .num_threads(thread_pool_threads)
            .thread_name(|i| format!("Worker thread #{i}"))
            .build()
            .unwrap(),
        metrics: metrics::Resources {
            frame_count: 0,
            frame_time: metrics::FrameTime {
                avg_fps: 60.0, // whatever
                avg_frametime_ms: 1000.0 / 60.0,
                frametime_history: [1000.0 / 60.0; 32],
                last_updated: now,
            },
        },
    }
}

pub fn update_pre(res: &mut Resources, event: &Event<()>) {
    let prev_t = res.time.secs_f32;

    let now = Instant::now();
    res.time.now = now;
    res.time.secs_f32 = (now - res.time.at_launch).as_secs_f32();
    res.time.ms_u32 = (now - res.time.at_launch).as_millis() as u32;
    res.time.dt_secs = res.time.secs_f32 - prev_t;

    let timings = &mut res.metrics.frame_time;
    let frametime = (now - timings.last_updated).as_secs_f32() * 1000.0;
    timings.frametime_history
        [res.metrics.frame_count as usize & (timings.frametime_history.len() - 1)] = frametime;

    let avg =
        timings.frametime_history.iter().sum::<f32>() / (timings.frametime_history.len() as f32);
    timings.avg_fps = 1000.0 / avg;
    timings.avg_frametime_ms = avg;
    timings.last_updated = now;

    res.metrics.frame_count += 1;

    Keyboard::tick(&mut res.input.keyboard);
    Mouse::first_tick(&mut res.input.mouse);

    if let Event::WindowEvent { event: WindowEvent::Resized(size), ..} = event {
        res.renderer.handle_window_resize((size.width, size.height));
    }
}

pub fn update_post(res: &mut Resources) {
    Mouse::last_tick(&mut res.input.mouse);
}