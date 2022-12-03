use log::info;
use views::{StateChange, View};
use winit::{
    event_loop::{ControlFlow, EventLoop}, event::{Event, WindowEvent},
};

pub mod game_view;
pub mod main_menu_view;
pub mod resources;
pub mod util;
pub mod views;
pub mod world;

fn main() {
    init_logger();

    let event_loop = EventLoop::new();

    let mut resources = resources::init_resources("Game", &event_loop);
    let mut view = View::main_menu();

    event_loop.run(move |event, _, flow| {
        resources::update_pre(&mut resources, &event);

        if let Event::LoopDestroyed | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            view.on_exit().unwrap();
            *flow = ControlFlow::Exit;
            return;
        }

        if let Some(change) = view.on_event(event) {
            process_state_change(*change, &mut view, flow);
        }

        resources::update_post(&mut resources);
    });
}

#[cold]
#[inline(never)]
fn process_state_change(change: StateChange, view: &mut Box<View>, flow: &mut ControlFlow) {
    match change {
        StateChange::Exit => *flow = ControlFlow::Exit,
        StateChange::SwitchTo(new_view) => {
            view.on_exit().unwrap();
            *view = new_view;
            view.on_enter().unwrap();
        }
    }
}

fn init_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{} : {}] {}",
                chrono::Local::now().format("%H:%M:%S"),
                &record.target()[record.target().find(':').map_or(0, |i| i + 2)..],
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log").unwrap())
        .apply()
        .unwrap();

    info!("Logger initialized");
}
