use winit::event::Event;

use crate::views::StateChange;


pub struct MainMenuView {
    
}

impl MainMenuView {
    pub fn new() -> Self {
        Self {
            
        }
    }
}

impl MainMenuView {
    pub fn on_enter_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_exit_view(&mut self) -> anyhow::Result<()> {

        Ok(())
    }

    pub fn on_event(&mut self, _event: Event<()>) -> Option<Box<StateChange>> {

        None
    }
}
