use crate::grid::FlatGrid;

// --- Game outcome ---

#[derive(Clone, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Won(usize), // player index
    Tie,
}

// --- Undo / redo history ---

// A single disk placement, stored so it can be undone or redone.
#[derive(Clone)]
pub struct MoveRecord {
    pub col: usize,
    pub row: usize,
    pub player: usize,
}

// Full board state captured before a soft reset, so the reset can be undone.
#[derive(Clone)]
pub struct ResetSnapshot {
    pub grid: FlatGrid,
    pub current_player: usize,
    pub state: GameState,
    pub scores: Vec<u32>,
}

// One entry on the history or redo stack.
#[derive(Clone)]
pub enum HistoryEntry {
    Move(MoveRecord),
    Reset(ResetSnapshot),
}

// --- UI feedback ---

// Shown above the board briefly after an undo or redo to confirm what happened.
#[derive(Clone)]
pub enum ActionBanner {
    Placement {
        player: usize,
        col: usize,
        is_redo: bool,
    },
    Reset {
        is_redo: bool,
    },
}
