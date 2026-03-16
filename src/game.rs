use std::io::Write;

use crossterm::{
    cursor::{MoveDown, MoveRight, MoveTo},
    queue,
    style::{Attribute, Print, SetAttribute},
};

use crate::animator::Animator;
use crate::disk::Disk;
use crate::grid::Grid;
use crate::sparse::{SparseSegment, render_sparse_line};
use crate::textures::Textures;
use crate::translate::Translate;
use crate::types::GameState;

// Owns the board grid, screen position, and game outcome state for one instance.
// Textures are fetched via Textures::get() and never stored.
pub struct Game {
    pub grid: Grid,
    pub translate: Translate,
    pub state: GameState,
}

impl Game {
    // --- Construction ---

    pub fn new() -> Self {
        let mut new = Self {
            grid: Grid::default(),
            translate: Translate::new(0, 0),
            state: GameState::Playing,
        };
        new.update_translate();
        new
    }

    // Recalculates screen position from current terminal size.
    pub fn update_translate(&mut self) {
        let textures = Textures::get();
        let (width, height) = crossterm::terminal::size().unwrap();
        let right = (width / 2).saturating_sub(textures.board_width / 2) as i32;
        let bottom = height.saturating_sub(textures.board_height) as i32;
        self.translate = Translate::new(right, bottom);
    }

    // --- Game logic ---

    // Places a disk, updates self.state, and returns an Animator.
    pub fn drop_animated(
        &mut self,
        disk: Disk,
        col: usize,
    ) -> Result<Animator, Box<dyn std::error::Error>> {
        let textures = Textures::get();
        let row = self.grid.push(disk, col)?;
        let animator = self
            .grid
            .animate_push(disk, col, row, textures, &self.translate)?;
        self.state = if self.grid.check_winner(col, row) {
            GameState::Won(disk.player as usize)
        } else if self.grid.is_full() {
            GameState::Tie
        } else {
            GameState::Playing
        };
        Ok(animator)
    }

    // --- Rendering ---

    // Clears the area above the board and blanks the board hole-positions,
    // so falling disks have a clean backdrop to animate through.
    pub fn display_whitespace(&self, stdout: &mut impl Write) -> std::io::Result<()> {
        let textures = Textures::get();
        let translate = &self.translate;

        for row in 0..translate.y {
            queue!(
                stdout,
                MoveTo(translate.x.max(0) as u16, row as u16),
                Print(&textures.whitespace),
            )?;
        }

        queue!(stdout, translate.move_to(0, 0))?;
        for segments in &textures.parsed_board_lines {
            for seg in segments {
                match seg {
                    SparseSegment::Text(s) => {
                        queue!(stdout, MoveRight(s.chars().count() as u16))?;
                    }
                    SparseSegment::Skip(n) => {
                        queue!(stdout, Print(" ".repeat(*n as usize)))?;
                    }
                }
            }
            queue!(stdout, translate.move_to_column(0), MoveDown(1))?;
        }
        Ok(())
    }

    // Draws all static (non-animating) disks, skipping cells with an active animator.
    pub fn display_disks(
        &self,
        stdout: &mut impl Write,
        skip: &[[bool; 6]; 7],
        is_active: bool,
    ) -> std::io::Result<()> {
        let textures = Textures::get();
        let translate = &self.translate;

        for (x, col_skip) in skip.iter().enumerate().take(self.grid.width) {
            let char_x = textures.char_x_positions[x];
            for (y, &is_skipped) in col_skip.iter().enumerate().take(self.grid.height) {
                if is_skipped {
                    continue;
                }
                if let Some(disk) = self.grid.get_disk(x, y) {
                    let char_y = (self.grid.height - y - 1) as i32 * textures.disk_height_i32;
                    disk.display(stdout, char_x, char_y, textures, translate)?;
                    if !is_active {
                        queue!(stdout, SetAttribute(Attribute::Dim))?;
                    }
                }
            }
        }
        Ok(())
    }

    // Draws the board overlay (walls, holes) on top of everything.
    pub fn display_board(&self, stdout: &mut impl Write) -> std::io::Result<()> {
        let textures = Textures::get();
        let translate = &self.translate;
        queue!(stdout, translate.move_to(0, 0))?;
        for segments in &textures.parsed_board_lines {
            render_sparse_line(stdout, segments)?;
            queue!(stdout, translate.move_to_column(0), MoveDown(1))?;
        }
        Ok(())
    }
}
