use std::sync::OnceLock;

use crate::constants::{BOARD_TEXTURE, DISK_TEXTURE};
use crate::sparse::{SparseSegment, parse_sparse_line};
use crate::strings::Strings;

static TEXTURES: OnceLock<Textures> = OnceLock::new();

// All texture data computed once at startup and shared as a static reference.
pub struct Textures {
    // --- Board dimensions ---
    pub board_width: u16,
    pub board_height: u16,

    // --- Disk dimensions ---
    pub disk_width: u16,
    pub disk_height: u16,
    pub disk_height_i32: i32, // pre-cast, used heavily in coordinate math

    // --- Per-column terminal x positions ---
    // char_x_positions[col] = terminal column where that board column starts
    pub char_x_positions: Vec<i32>,

    // --- Pre-parsed line segments (avoid re-scanning statics every frame) ---
    pub parsed_board_lines: Vec<Vec<SparseSegment>>,
    pub parsed_disk_lines: Vec<Vec<SparseSegment>>,

    // --- Pre-built strings ---
    pub whitespace: String, // board_width spaces, used to blank above the board
    pub col_cursor: String, // disk_width cursor chars, used as the column indicator
}

impl Textures {
    pub fn get() -> &'static Self {
        TEXTURES.get_or_init(|| {
            // Parse column positions from the header line of the board texture
            let header = BOARD_TEXTURE
                .lines()
                .next()
                .expect("board texture is empty");
            let board_columns: Vec<u16> = header
                .chars()
                .enumerate()
                .filter(|(_, c)| c.is_ascii_digit())
                .map(|(i, _)| i.try_into().expect("column index too large"))
                .collect();

            let board_width: u16 = BOARD_TEXTURE
                .lines()
                .map(|line| line.chars().count())
                .max()
                .expect("board texture has no lines")
                .try_into()
                .expect("board width too large");

            // Subtract 1 to exclude the header line from the rendered height
            let board_height: u16 = BOARD_TEXTURE
                .lines()
                .count()
                .saturating_sub(1)
                .try_into()
                .expect("board height too large");

            let disk_width: u16 = DISK_TEXTURE
                .lines()
                .map(|line| line.chars().count())
                .max()
                .expect("disk texture has no lines")
                .try_into()
                .expect("disk width too large");

            let disk_height: u16 = DISK_TEXTURE
                .lines()
                .count()
                .try_into()
                .expect("disk height too large");

            let board_lines: Vec<&'static str> = BOARD_TEXTURE.lines().skip(1).collect();
            let disk_lines: Vec<&'static str> = DISK_TEXTURE.lines().collect();

            let mut textures = Self {
                board_width,
                board_height,
                disk_width,
                disk_height,
                disk_height_i32: i32::from(disk_height),
                char_x_positions: board_columns.iter().map(|&c| i32::from(c)).collect(),
                parsed_board_lines: board_lines
                    .iter()
                    .map(|line| parse_sparse_line(line))
                    .collect(),
                parsed_disk_lines: disk_lines
                    .iter()
                    .map(|line| parse_sparse_line(line))
                    .collect(),
                whitespace: " ".repeat(board_width as usize),
                col_cursor: String::new(),
            };

            let cursor_char = Strings::get().raw("ui.col_cursor.char");
            textures.col_cursor = cursor_char.repeat(textures.disk_width as usize);
            textures
        })
    }
}
