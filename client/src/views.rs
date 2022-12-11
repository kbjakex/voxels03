use winit::event::Event;

use crate::{main_menu_view::MainMenuView, game_view::GameView, resources::Resources};

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

macro_rules! switch {
    ($what:expr, $s:tt => $f:expr) => {
        return match $what {
            View::MainMenu($s) => $f,
            View::Game($s) => $f,
        }
    };
}

impl View {
    pub fn on_enter(&mut self, res: &mut Resources) -> anyhow::Result<()> {
        switch!(self, state => state.on_enter_view(res));
    }

    pub fn on_exit(&mut self, res: &mut Resources) -> anyhow::Result<()> {
        switch!(self, state => state.on_exit_view(res));
    }

    pub fn on_update(&mut self, res: &mut Resources) -> Option<Box<StateChange>> {
        switch!(self, state => state.on_update(res));
    }

    pub fn on_event(&mut self, event: Event<()>, res: &mut Resources) -> Option<Box<StateChange>> {
        switch!(self, state => state.on_event(event, res));
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
