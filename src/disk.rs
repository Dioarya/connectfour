use std::io::Write;

use crossterm::{
    cursor::MoveDown,
    queue,
    style::{Color, ResetColor, SetForegroundColor},
};

use crate::sparse::render_sparse_line;
use crate::textures::Textures;
use crate::translate::Translate;

// Packed cell value used in Grid::cells.
// 0 = empty, 1 = player 0, 2 = player 1.
// Fits in a u8, keeping the hot grid at 42 bytes flat.
pub type Cell = u8;
pub const CELL_EMPTY: Cell = 0;

// A disk ready for display — constructed on demand from a Cell value.
// player is 0-indexed; color is the RGB triple for that player.
#[derive(Clone, Copy)]
pub struct Disk {
    pub player: u8,
    pub color: (u8, u8, u8),
}

impl Disk {
    pub fn display(
        &self,
        stdout: &mut impl Write,
        x: i32,
        y: i32,
        textures: &Textures,
        translate: &Translate,
    ) -> std::io::Result<()> {
        let (r, g, b) = self.color;
        queue!(
            stdout,
            SetForegroundColor(Color::Rgb { r, g, b }),
            translate.move_to(x, y),
        )?;
        for segments in &textures.parsed_disk_lines {
            render_sparse_line(stdout, segments)?;
            queue!(stdout, translate.move_to_column(x), MoveDown(1))?;
        }
        queue!(stdout, ResetColor)?;
        Ok(())
    }
}
