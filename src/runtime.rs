use std::io::Write;

use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};

use crate::animator::Animator;
use crate::config::Config;
use crate::disk::Disk;
use crate::game::Game;
use crate::grid::{GRID_COLS, GRID_ROWS, Grid};
use crate::strings::Strings;
use crate::textures::Textures;
use crate::translate::Translate;
use crate::types::{ActionBanner, GameState, HistoryEntry, ResetSnapshot};

// All mutable state for one game instance.
// game.state holds the current outcome; all other runtime state lives here.
pub struct Runtime {
    // --- Core game ---
    pub game: Game,
    pub current_player: usize,
    pub selected_col: usize,

    // --- Scoring ---
    pub scores: Vec<u32>,
    pub score_strings: Vec<String>, // cached; rebuilt only on score change

    // --- Animation ---
    // Fixed 2D array indexed [col][row] — O(1) access and removal, zero allocation.
    pub animators: [[Option<Animator>; GRID_ROWS]; GRID_COLS],

    // --- Confirm prompts ---
    pub confirm_reset: bool,

    // --- Undo / redo ---
    pub history: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,

    // --- UI feedback ---
    pub action_banner: Option<ActionBanner>,

    // --- Cursor ---
    pub show_cursor: bool,

    // --- Dirty flag ---
    pub dirty: bool,
}

impl Runtime {
    // --- Construction ---

    pub fn new() -> Self {
        let scores = vec![0u32; Config::get().player_colors.len()];
        let score_strings = Self::build_score_strings(&scores);
        Self {
            game: Game::new(),
            current_player: 0,
            selected_col: 0,
            scores,
            score_strings,
            animators: std::array::from_fn(|_| std::array::from_fn(|_| None)),
            confirm_reset: false,
            history: Vec::new(),
            redo_stack: Vec::new(),
            action_banner: None,
            show_cursor: true,
            dirty: true,
        }
    }

    // --- Score helpers ---

    fn build_score_strings(scores: &[u32]) -> Vec<String> {
        let strings = Strings::get();
        scores
            .iter()
            .enumerate()
            .map(|(i, &score)| {
                strings.fmt("score.entry", &[&(i + 1).to_string(), &score.to_string()])
            })
            .collect()
    }

    pub fn update_score_strings(&mut self) {
        self.score_strings = Self::build_score_strings(&self.scores);
    }

    // --- Animator helpers ---

    fn clear_animators(&mut self) {
        for col in &mut self.animators {
            for cell in col.iter_mut() {
                *cell = None;
            }
        }
    }

    // --- Reset ---

    // Wipes everything including scores and history. No undo possible after this.
    pub fn hard_reset(&mut self) {
        let translate_x = self.game.translate.x;
        let translate_y = self.game.translate.y;
        self.game = Game::new();
        self.game.translate = Translate::new(translate_x, translate_y);
        self.current_player = 0;
        self.selected_col = 0;
        self.scores = vec![0; Config::get().player_colors.len()];
        self.update_score_strings();
        self.clear_animators();
        self.confirm_reset = false;
        self.history.clear();
        self.redo_stack.clear();
        self.action_banner = None;
        self.show_cursor = true;
        self.dirty = true;
    }

    // Soft reset: snapshots current state to history so it can be undone.
    pub fn reset_game(&mut self) {
        let snapshot = ResetSnapshot {
            grid: self.game.grid.to_flat(),
            current_player: self.current_player,
            state: self.game.state.clone(),
            scores: self.scores.clone(),
        };
        self.history.push(HistoryEntry::Reset(snapshot));
        self.redo_stack.clear();

        let translate_x = self.game.translate.x;
        let translate_y = self.game.translate.y;
        self.game = Game::new();
        self.game.translate = Translate::new(translate_x, translate_y);
        self.current_player = 0;
        self.selected_col = 0;
        self.clear_animators();
        self.confirm_reset = false;
        self.action_banner = None;
        self.show_cursor = true;
        self.dirty = true;
    }

    // --- Undo / redo ---

    pub fn undo(&mut self) {
        self.confirm_reset = false;
        let entry = match self.history.pop() {
            Some(e) => e,
            None => return,
        };

        match entry {
            HistoryEntry::Move(record) => {
                self.animators[record.col][record.row] = None;
                self.game.grid.remove(record.col, record.row);

                if let GameState::Won(p) = self.game.state
                    && self.scores[p] > 0
                {
                    self.scores[p] -= 1;
                    self.update_score_strings();
                }

                self.game.state = GameState::Playing;
                self.current_player = record.player;
                self.redo_stack.push(HistoryEntry::Move(record.clone()));
                self.action_banner = Some(ActionBanner::Placement {
                    player: record.player,
                    col: record.col,
                    is_redo: false,
                });
            }
            HistoryEntry::Reset(snapshot) => {
                // Capture current (post-reset) state so redo can return here
                let redo_snapshot = ResetSnapshot {
                    grid: self.game.grid.to_flat(),
                    current_player: self.current_player,
                    state: self.game.state.clone(),
                    scores: self.scores.clone(),
                };
                self.redo_stack.push(HistoryEntry::Reset(redo_snapshot));

                let translate_x = self.game.translate.x;
                let translate_y = self.game.translate.y;
                self.game = Game::new();
                self.game.translate = Translate::new(translate_x, translate_y);
                self.game.grid = Grid::from_flat(&snapshot.grid);
                self.game.state = snapshot.state;
                self.current_player = snapshot.current_player;
                self.scores = snapshot.scores;
                self.update_score_strings();
                self.clear_animators();
                self.confirm_reset = false;
                self.action_banner = Some(ActionBanner::Reset { is_redo: false });
            }
        }
        self.dirty = true;
    }

    pub fn redo(&mut self) {
        self.confirm_reset = false;
        let entry = match self.redo_stack.pop() {
            Some(e) => e,
            None => return,
        };

        match entry {
            HistoryEntry::Move(record) => {
                let (r, g, b) = Config::get().player_colors[record.player];
                let disk = Disk {
                    player: record.player as u8,
                    color: (r, g, b),
                };

                self.game.grid.cells[record.col][record.row] = disk.player + 1;
                if self.game.grid.col_fill[record.col] <= record.row {
                    self.game.grid.col_fill[record.col] = record.row + 1;
                }

                self.game.state = if self.game.grid.check_winner(record.col, record.row) {
                    GameState::Won(record.player)
                } else if self.game.grid.is_full() {
                    GameState::Tie
                } else {
                    GameState::Playing
                };

                if let GameState::Won(p) = self.game.state {
                    self.scores[p] += 1;
                    self.update_score_strings();
                }

                if self.game.state == GameState::Playing {
                    self.current_player = 1 - record.player;
                }

                if let Ok(animator) = self.game.grid.animate_push(
                    disk,
                    record.col,
                    record.row,
                    Textures::get(),
                    &self.game.translate,
                ) {
                    self.animators[record.col][record.row] = Some(animator);
                }

                self.action_banner = Some(ActionBanner::Placement {
                    player: record.player,
                    col: record.col,
                    is_redo: true,
                });
                self.selected_col = record.col;
                self.show_cursor = false;
                self.history.push(HistoryEntry::Move(record));
            }
            HistoryEntry::Reset(redo_snapshot) => {
                // Save current (pre-redo) state so undo can return here
                let snapshot = ResetSnapshot {
                    grid: self.game.grid.to_flat(),
                    current_player: self.current_player,
                    state: self.game.state.clone(),
                    scores: self.scores.clone(),
                };
                self.history.push(HistoryEntry::Reset(snapshot));

                let translate_x = self.game.translate.x;
                let translate_y = self.game.translate.y;
                self.game = Game::new();
                self.game.translate = Translate::new(translate_x, translate_y);
                self.game.grid = Grid::from_flat(&redo_snapshot.grid);
                self.game.state = redo_snapshot.state;
                self.current_player = redo_snapshot.current_player;
                self.scores = redo_snapshot.scores;
                self.update_score_strings();
                self.clear_animators();
                self.confirm_reset = false;
                self.action_banner = Some(ActionBanner::Reset { is_redo: true });
            }
        }
        self.dirty = true;
    }

    // --- Rendering ---

    pub fn display_status(
        &self,
        stdout: &mut impl Write,
        is_active: bool,
        term_size: (u16, u16),
    ) -> std::io::Result<()> {
        let textures = Textures::get();
        let strings = Strings::get();
        let (_, term_height) = term_size;
        let translate = &self.game.translate;
        let board_center_x = translate.x + i32::from(textures.board_width) / 2;

        let turn_y = (translate.y - 5).max(0) as u16;
        let undo_y = (translate.y - 3).max(0) as u16;
        let status_y = (translate.y - 1).max(0) as u16;

        let padded = |msg: String| format!(" {msg} ");

        // Re-applies Dim after ResetColor when the instance is inactive,
        // since ResetColor clears all SGR attributes including Dim.
        macro_rules! color_print {
            ($r:expr, $g:expr, $b:expr, $msg:expr) => {{
                queue!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: $r,
                        g: $g,
                        b: $b,
                    }),
                    Print($msg),
                    ResetColor,
                )?;
                if !is_active {
                    queue!(stdout, SetAttribute(Attribute::Dim))?;
                }
            }};
        }

        // Column selection cursor — shown only when playing and active
        if is_active
            && self.show_cursor
            && self.game.state == GameState::Playing
            && !self.confirm_reset
            && self.selected_col < textures.char_x_positions.len()
        {
            let (r, g, b) = Config::get().player_colors[self.current_player];
            let cursor_x =
                (translate.x + textures.char_x_positions[self.selected_col]).max(0) as u16;
            queue!(stdout, MoveTo(cursor_x, status_y))?;
            color_print!(r, g, b, &textures.col_cursor);
        }

        // Action banner (undo / redo feedback)
        if let Some(banner) = &self.action_banner {
            match banner {
                ActionBanner::Placement {
                    player,
                    col,
                    is_redo,
                } => {
                    let (r, g, b) = Config::get().player_colors[*player];
                    let key = if *is_redo {
                        "action.placement.redo"
                    } else {
                        "action.placement.undo"
                    };
                    let full =
                        strings.fmt(key, &[&(player + 1).to_string(), &(col + 1).to_string()]);
                    let prefix = format!(" Player {}", player + 1);
                    let suffix = format!("{} ", &full[prefix.trim_start().len()..]);
                    let x =
                        (board_center_x - (prefix.len() + suffix.len()) as i32 / 2).max(0) as u16;
                    queue!(stdout, MoveTo(x, undo_y))?;
                    color_print!(r, g, b, &prefix);
                    queue!(stdout, Print(&suffix))?;
                }
                ActionBanner::Reset { is_redo } => {
                    let msg = padded(strings.fmt(
                        if *is_redo {
                            "action.reset.redo"
                        } else {
                            "action.reset.undo"
                        },
                        &[],
                    ));
                    let x = (board_center_x - msg.len() as i32 / 2).max(0) as u16;
                    queue!(stdout, MoveTo(x, undo_y), Print(&msg))?;
                }
            }
        }

        // Turn indicator / confirm prompt / end-game status
        match self.game.state {
            GameState::Playing if self.confirm_reset => {
                if !is_active {
                    return Ok(());
                }
                let msg = padded(strings.fmt("confirm.reset", &[]));
                let x = (board_center_x - msg.len() as i32 / 2).max(0) as u16;
                queue!(stdout, MoveTo(x, turn_y), Print(&msg))?;
            }
            GameState::Playing => {
                let (r, g, b) = Config::get().player_colors[self.current_player];
                let msg =
                    padded(strings.fmt("player.turn", &[&(self.current_player + 1).to_string()]));
                let x = (board_center_x - msg.len() as i32 / 2).max(0) as u16;
                queue!(stdout, MoveTo(x, turn_y))?;
                color_print!(r, g, b, &msg);
            }
            GameState::Won(p) => {
                let (r, g, b) = Config::get().player_colors[p];
                let msg = padded(strings.fmt("player.wins", &[&(p + 1).to_string()]));
                let x = (board_center_x - msg.len() as i32 / 2).max(0) as u16;
                queue!(stdout, MoveTo(x, status_y))?;
                color_print!(r, g, b, &msg);
            }
            GameState::Tie => {
                let msg = padded(strings.fmt("player.tie", &[]));
                let x = (board_center_x - msg.len() as i32 / 2).max(0) as u16;
                queue!(stdout, MoveTo(x, status_y), Print(&msg))?;
            }
        }

        // Scoreboard — always shown
        let separator = strings.raw("score.separator");
        let total_len = self
            .score_strings
            .iter()
            .map(std::string::String::len)
            .sum::<usize>()
            + separator.len() * self.score_strings.len().saturating_sub(1)
            + 2;
        let x = (board_center_x - total_len as i32 / 2).max(0) as u16;
        let score_y = term_height.saturating_sub(1);
        queue!(stdout, MoveTo(x, score_y), Print(" "))?;
        for (i, part) in self.score_strings.iter().enumerate() {
            let (r, g, b) = Config::get().player_colors[i];
            color_print!(r, g, b, part);
            if i < self.score_strings.len() - 1 {
                queue!(stdout, Print(separator))?;
            }
        }
        queue!(stdout, Print(" "))?;

        Ok(())
    }

    pub fn draw(
        &self,
        stdout: &mut impl Write,
        is_active: bool,
        dim: bool,
        term_size: (u16, u16),
    ) -> std::io::Result<()> {
        // Build skip mask from the fixed animator array — O(1), zero allocation
        let skip: [[bool; GRID_ROWS]; GRID_COLS] = std::array::from_fn(|col| {
            std::array::from_fn(|row| self.animators[col][row].is_some())
        });

        if dim {
            queue!(stdout, SetAttribute(Attribute::Dim))?;
        }

        self.game.display_whitespace(stdout)?;
        self.game.display_disks(stdout, &skip, is_active && !dim)?;

        for col in 0..GRID_COLS {
            for row in 0..GRID_ROWS {
                if let Some(animator) = &self.animators[col][row] {
                    animator.display(stdout, Textures::get(), &self.game.translate)?;
                    if dim {
                        queue!(stdout, SetAttribute(Attribute::Dim))?;
                    }
                }
            }
        }

        self.game.display_board(stdout)?;
        self.display_status(stdout, is_active && !dim, term_size)?;

        if dim {
            queue!(stdout, SetAttribute(Attribute::NormalIntensity))?;
        }

        Ok(())
    }
}
