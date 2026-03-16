use crate::render::update_all_translates;
use crate::runtime::Runtime;

// Top-level application state: the collection of game instances and which one is active.
pub struct AppState {
    pub instances: Vec<Runtime>,
    pub active: usize,
    pub confirm_remove: bool,
}

impl AppState {
    pub fn new() -> Self {
        let mut app = Self {
            instances: vec![Runtime::new()],
            active: 0,
            confirm_remove: false,
        };
        update_all_translates(&mut app.instances);
        app
    }

    pub fn active_mut(&mut self) -> &mut Runtime {
        &mut self.instances[self.active]
    }
}
