use crate::animator::Animator;
use crate::config::Config;
use crate::disk::{CELL_EMPTY, Cell, Disk};
use crate::textures::Textures;
use crate::translate::Translate;

pub const GRID_COLS: usize = 7;
pub const GRID_ROWS: usize = 6;

// The hot-path grid used during play.
// cells is column-major: cells[col][row], row 0 = bottom.
// col_fill tracks how many disks are in each column for O(1) push/full checks.
#[derive(Clone)]
pub struct Grid {
    // Packed u8 cells — 0 = empty, 1 = P0, 2 = P1
    pub cells: Vec<Vec<Cell>>,
    // Next empty row per column; col_fill[c] == height means column c is full
    pub col_fill: [usize; GRID_COLS],
    pub width: usize,
    pub height: usize,
}

// Compact single-allocation snapshot used for undo/redo history.
#[derive(Clone)]
pub struct FlatGrid {
    pub cells: Vec<Cell>,
    pub col_fill: [usize; GRID_COLS],
    pub width: usize,
    pub height: usize,
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(GRID_COLS, GRID_ROWS)
    }
}

impl Grid {
    // --- Construction ---

    pub fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![vec![CELL_EMPTY; height]; width],
            col_fill: [0; GRID_COLS],
            width,
            height,
        }
    }

    pub fn from_flat(flat: &FlatGrid) -> Self {
        let cells = flat.cells.chunks(flat.height).map(<[u8]>::to_vec).collect();
        Self {
            cells,
            col_fill: flat.col_fill,
            width: flat.width,
            height: flat.height,
        }
    }

    // --- Queries ---

    #[inline]
    pub const fn is_column_full(&self, col: usize) -> bool {
        self.col_fill[col] >= self.height
    }

    pub fn is_full(&self) -> bool {
        self.col_fill
            .iter()
            .take(self.width)
            .all(|&f| f >= self.height)
    }

    #[inline]
    pub fn get_cell(&self, col: usize, row: usize) -> Cell {
        self.cells[col][row]
    }

    // Returns a display-ready Disk from a cell, or None if empty.
    #[inline]
    pub fn get_disk(&self, col: usize, row: usize) -> Option<Disk> {
        let cell = self.cells[col][row];
        if cell == CELL_EMPTY {
            return None;
        }
        let player = (cell - 1) as usize;
        Some(Disk {
            player: player as u8,
            color: Config::get().player_colors[player],
        })
    }

    // Returns true if the disk at (col, row) completes a connect four.
    pub fn check_winner(&self, col: usize, row: usize) -> bool {
        let cell = self.get_cell(col, row);
        if cell == CELL_EMPTY {
            return false;
        }

        let dirs: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
        let (grid_width, grid_height) = (self.width as i32, self.height as i32);

        for (dc, dr) in dirs {
            let mut count = 1i32;
            for sign in [-1i32, 1] {
                let mut c = col as i32 + dc * sign;
                let mut r = row as i32 + dr * sign;
                while c >= 0
                    && c < grid_width
                    && r >= 0
                    && r < grid_height
                    && self.cells[c as usize][r as usize] == cell
                {
                    count += 1;
                    if count >= 4 {
                        return true;
                    }
                    c += dc * sign;
                    r += dr * sign;
                }
            }
            if count >= 4 {
                return true;
            }
        }
        false
    }

    // --- Mutation ---

    // Places a disk in a column and returns the row it landed in.
    pub fn push(&mut self, disk: Disk, col: usize) -> Result<usize, String> {
        if col >= self.width {
            return Err("Invalid column".to_string());
        }
        if self.is_column_full(col) {
            return Err("Column full".to_string());
        }
        let row = self.col_fill[col];
        self.cells[col][row] = disk.player + 1;
        self.col_fill[col] += 1;
        Ok(row)
    }

    // Removes the disk at (col, row). row must be the topmost disk in that column.
    pub fn remove(&mut self, col: usize, row: usize) {
        self.cells[col][row] = CELL_EMPTY;
        if self.col_fill[col] > 0 && row == self.col_fill[col] - 1 {
            self.col_fill[col] -= 1;
        }
    }

    // --- Animation ---

    pub fn animate_push(
        &self,
        disk: Disk,
        col: usize,
        row: usize,
        textures: &Textures,
        translate: &Translate,
    ) -> Result<Animator, String> {
        let target_y = (self.height - row - 1) as f64 * f64::from(textures.disk_height);
        let start_y = -f64::from(translate.y);
        Ok(Animator::new(disk, col, row, start_y, target_y))
    }

    // --- Serialisation ---

    pub fn to_flat(&self) -> FlatGrid {
        FlatGrid {
            cells: self.cells.iter().flatten().copied().collect(),
            col_fill: self.col_fill,
            width: self.width,
            height: self.height,
        }
    }
}
