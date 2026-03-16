// --- Texture file paths (compiled in at build time) ---

pub const BOARD_TEXTURE: &str = include_str!("board.txt");
pub const DISK_TEXTURE: &str = include_str!("disk.txt");

// --- Game configuration ---

pub const PLAYER_COLORS: [(u8, u8, u8); 2] = [(220, 40, 40), (220, 200, 0)];
pub const INSTANCE_GAP: u16 = 2;
