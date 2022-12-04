use winit::event::Event;

use crate::{main_menu_view::MainMenuView, game_view::GameView};

pub enum StateChange {
    Exit,
    SwitchTo(Box<View>)
}

pub fn exit() -> Option<Box<StateChange>> {
    Some(Box::new(StateChange::Exit))
}

pub fn switch_to(view: Box<View>) -> Option<Box<StateChange>> {
    Some(Box::new(StateChange::SwitchTo(view)))
}

pub enum View {
    MainMenu(MainMenuView),
    Game(GameView),
}

impl View {
    pub fn main_menu() -> Box<View> {
        Box::new(View::MainMenu(MainMenuView::new()))
    }

    pub fn game() -> Box<View> {
        Box::new(View::Game(GameView::new()))
    }
}

impl View {
    pub fn on_enter(&mut self) -> anyhow::Result<()> {
        match self {
            View::MainMenu(state) => state.on_enter_view(),
            View::Game(state) => state.on_enter_view(),
        }
    }
    
    pub fn on_exit(&mut self) -> anyhow::Result<()> {
        match self {
            View::MainMenu(state) => state.on_exit_view(),
            View::Game(state) => state.on_exit_view(),
        }
    }

    pub fn on_event(&mut self, event: Event<()>) -> Option<Box<StateChange>> {
        match self {
            View::MainMenu(state) => state.on_event(event),
            View::Game(state) => state.on_event(event),
        }
    }
}

// Note about `on_event()` returning a gnarly `Option<Box<StateView>>`,
// where SwitchTo also includes a Boxed value, seemingly double-boxing:
//
// This is optimized for the common case of None being returned.
//
// By returning a Option<Box<..>>, the return value fits in a single
// register (std::mem::size_of::<Option<Box<T>>>() == 8) and there are no 
// allocations, and a simple compare-zero is sufficient. 
//
// Because of how incredibly rare it is for a non-None value to be
// returned compared to None being returned dozens if not hundreds
// of times every single frame, this is certainly the right tradeoff 
// to make.
