use crossterm::cursor::{MoveTo, MoveToColumn};

// Maps board-relative coordinates to terminal-absolute coordinates.
// Every board instance has its own Translate so boards can be positioned independently.
pub struct Translate {
    pub x: i32,
    pub y: i32,
}

impl Translate {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn move_to(&self, x: i32, y: i32) -> MoveTo {
        MoveTo((x + self.x).max(0) as u16, (y + self.y).max(0) as u16)
    }

    pub fn move_to_column(&self, x: i32) -> MoveToColumn {
        MoveToColumn((x + self.x).max(0) as u16)
    }
}
