use std::sync::OnceLock;

use crossterm::event::KeyCode;

static CONFIG: OnceLock<Config> = OnceLock::new();

// All runtime-configurable settings, parsed once from CLI args at startup.
pub struct Config {
    pub keybinds: KeyBinds,
    pub fps: u64,
    pub gravity: f64,
    pub coeff_restitution: f64,
    pub settle_threshold: f64,
    pub starting_instances: usize,
    pub player_colors: [(u8, u8, u8); 2],
    pub vfr: bool,
}

// One key per bindable action. Each is a Vec to allow multiple keys per action.
pub struct KeyBinds {
    pub move_left: Vec<KeyCode>,
    pub move_right: Vec<KeyCode>,
    pub drop: Vec<KeyCode>,
    pub undo: Vec<KeyCode>,
    pub redo: Vec<KeyCode>,
    pub reset: Vec<KeyCode>,
    pub quit: Vec<KeyCode>,
}

impl Default for KeyBinds {
    fn default() -> Self {
        Self {
            move_left: vec![KeyCode::Left, KeyCode::Char('a')],
            move_right: vec![KeyCode::Right, KeyCode::Char('d')],
            drop: vec![KeyCode::Enter, KeyCode::Char(' ')],
            undo: vec![KeyCode::Char('u')],
            redo: vec![KeyCode::Char('U')],
            reset: vec![KeyCode::Char('r')],
            quit: vec![KeyCode::Char('q')],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        use crate::constants::PLAYER_COLORS;
        Self {
            keybinds: KeyBinds::default(),
            fps: 60,
            gravity: 200.0,
            coeff_restitution: 0.5,
            settle_threshold: 24.0,
            starting_instances: 1,
            player_colors: PLAYER_COLORS,
            vfr: false,
        }
    }
}

impl Config {
    pub fn get() -> &'static Self {
        CONFIG.get_or_init(Self::from_args)
    }

    fn from_args() -> Self {
        let mut config = Self::default();
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--vfr" => {
                    config.vfr = true;
                }
                "--fps" => {
                    if let Some(val) = iter.next() {
                        if let Ok(fps) = val.parse::<u64>() {
                            config.fps = fps.max(1);
                        } else {
                            eprintln!("Invalid value for --fps: {val}");
                        }
                    }
                }
                "--gravity" => {
                    if let Some(val) = iter.next() {
                        if let Ok(gravity) = val.parse::<f64>() {
                            config.gravity = gravity;
                        } else {
                            eprintln!("Invalid value for --gravity: {val}");
                        }
                    }
                }
                "--restitution" => {
                    if let Some(val) = iter.next() {
                        if let Ok(r) = val.parse::<f64>() {
                            config.coeff_restitution = r.clamp(0.0, 1.0);
                        } else {
                            eprintln!("Invalid value for --restitution: {val}");
                        }
                    }
                }
                "--settle" => {
                    if let Some(val) = iter.next() {
                        if let Ok(threshold) = val.parse::<f64>() {
                            config.settle_threshold = threshold;
                        } else {
                            eprintln!("Invalid value for --settle: {val}");
                        }
                    }
                }
                "--instances" => {
                    if let Some(val) = iter.next() {
                        if let Ok(n) = val.parse::<usize>() {
                            config.starting_instances = n.clamp(1, 9);
                        } else {
                            eprintln!("Invalid value for --instances: {val}");
                        }
                    }
                }
                "--player-color" => {
                    if let Some(val) = iter.next()
                        && let Some((idx_str, rgb_str)) = val.split_once('=') {
                            if let (Ok(idx), Some(color)) =
                                (idx_str.parse::<usize>(), parse_color(rgb_str))
                            {
                                if (1..=2).contains(&idx) {
                                    config.player_colors[idx - 1] = color;
                                } else {
                                    eprintln!("Player index must be 1 or 2");
                                }
                            } else {
                                eprintln!("Invalid value for --player-color: {val}");
                                eprintln!("Expected format: --player-color 1=255,0,0");
                            }
                        }
                }
                "--bind" => {
                    if let Some(val) = iter.next() {
                        if let Some((action, key_str)) = val.split_once('=') {
                            let keys: Vec<KeyCode> =
                                key_str.split(',').filter_map(parse_key).collect();
                            if keys.is_empty() {
                                eprintln!("No valid keys for --bind {action}: {key_str}");
                            } else {
                                match action {
                                    "move-left" => config.keybinds.move_left = keys,
                                    "move-right" => config.keybinds.move_right = keys,
                                    "drop" => config.keybinds.drop = keys,
                                    "undo" => config.keybinds.undo = keys,
                                    "redo" => config.keybinds.redo = keys,
                                    "reset" => config.keybinds.reset = keys,
                                    "quit" => config.keybinds.quit = keys,
                                    _ => eprintln!("Unknown action for --bind: {action}"),
                                }
                            }
                        } else {
                            eprintln!("Invalid --bind format: {val}");
                            eprintln!("Expected format: --bind move-left=h,left");
                        }
                    }
                }
                unknown => {
                    eprintln!("Unknown argument: {unknown}");
                    eprintln!("Run with --help for usage.");
                }
            }
        }

        config
    }
}

fn parse_key(s: &str) -> Option<KeyCode> {
    match s.trim().to_lowercase().as_str() {
        "enter" => Some(KeyCode::Enter),
        "space" => Some(KeyCode::Char(' ')),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "esc" | "escape" => Some(KeyCode::Esc),
        "backspace" => Some(KeyCode::Backspace),
        "tab" => Some(KeyCode::Tab),
        s if s.len() == 1 => s.chars().next().map(KeyCode::Char),
        _ => {
            eprintln!("Unknown key name: {s}");
            None
        }
    }
}

fn parse_color(s: &str) -> Option<(u8, u8, u8)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let r = parts[0].trim().parse::<u8>().ok()?;
    let g = parts[1].trim().parse::<u8>().ok()?;
    let b = parts[2].trim().parse::<u8>().ok()?;
    Some((r, g, b))
}

fn print_help() {
    println!("connectfour — terminal Connect Four");
    println!();
    println!("USAGE:");
    println!("    connectfour [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!(
        "    --vfr                      Variable frame rate — redraw only when content changes"
    );
    println!("    --gravity <f>              Disk fall gravity in rows/s² (default: 200)");
    println!("    --restitution <f>          Bounce restitution 0.0–1.0 (default: 0.5)");
    println!("    --settle <f>               Velocity threshold to snap to rest (default: 24)");
    println!("    --instances <n>            Starting number of boards 1–9 (default: 1)");
    println!("    --player-color <n>=<r,g,b> Override player color, e.g. 1=255,0,0");
    println!("    --bind <action>=<keys>     Rebind an action, e.g. --bind undo=z,backspace");
    println!("    --help                     Print this help message");
    println!();
    println!("BINDABLE ACTIONS:");
    println!("    move-left   move-right   drop   undo   redo   reset   quit");
    println!();
    println!("KEY NAMES:");
    println!("    Single letters (a-z, A-Z, 0-9), or:");
    println!("    enter  space  left  right  up  down  esc  backspace  tab");
}
