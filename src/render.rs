use std::io::{BufWriter, Write};

use crossterm::{cursor::MoveTo, queue, style::Print};

use crate::app::AppState;
use crate::constants::INSTANCE_GAP;
use crate::runtime::Runtime;
use crate::strings::Strings;
use crate::textures::Textures;
use crate::translate::Translate;

// --- Synchronized output (eliminates flicker on supporting terminals) ---

pub fn begin_sync(stdout: &mut impl Write) -> std::io::Result<()> {
    stdout.write_all(b"\x1b[?2026h")
}

pub fn end_sync(stdout: &mut impl Write) -> std::io::Result<()> {
    stdout.write_all(b"\x1b[?2026l")
}

// --- Layout ---

// Recalculates every instance's screen position so they tile horizontally
// centered in the terminal. Called on init, resize, and instance add/remove.
pub fn update_all_translates(instances: &mut [Runtime]) {
    if instances.is_empty() {
        return;
    }
    let (term_width, term_height) = crossterm::terminal::size().unwrap();
    let textures = Textures::get();
    let instance_count = instances.len() as i32;
    let total_width =
        instance_count * textures.board_width as i32 + (instance_count - 1) * INSTANCE_GAP as i32;
    let start_x = ((term_width as i32 - total_width) / 2).max(0);
    let y = (term_height as i32 - textures.board_height as i32).max(0);

    for (i, inst) in instances.iter_mut().enumerate() {
        let x = start_x + i as i32 * (textures.board_width as i32 + INSTANCE_GAP as i32);
        inst.game.translate = Translate::new(x, y);
        inst.dirty = true;
    }
}

// Returns whether there is horizontal space for one more board.
pub fn can_add_instance(instances: &[Runtime]) -> bool {
    if instances.len() >= 9 {
        return false;
    }
    let (term_width, _) = crossterm::terminal::size().unwrap();
    let textures = Textures::get();
    let instance_count = (instances.len() + 1) as u16;
    let total_width = instance_count * textures.board_width + (instance_count - 1) * INSTANCE_GAP;
    total_width <= term_width
}

// --- Overlays ---

// Draws a centered box dialog asking for remove confirmation.
pub fn display_remove_confirm(
    stdout: &mut impl Write,
    index: usize,
    term_size: (u16, u16),
) -> std::io::Result<()> {
    let (term_width, term_height) = term_size;
    let raw_msg = Strings::get().fmt("confirm.remove", &[&(index + 1).to_string()]);
    let raw_msg_width = raw_msg
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let h_bars: String = std::iter::repeat_n('─', raw_msg_width).collect();
    let barred_msg = raw_msg
        .lines()
        .map(|line| format!("│{}│\n", line))
        .collect::<String>()
        .trim_end()
        .to_string();
    let msg = format!("┌{}┐\n{}\n└{}┘", h_bars, barred_msg, h_bars);

    let msg_width = msg
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let msg_height = msg.lines().count();
    let x = ((term_width as usize).saturating_sub(msg_width) / 2) as u16;
    let y = ((term_height as usize).saturating_sub(msg_height) / 2) as u16;

    for (j, line) in msg.lines().enumerate() {
        queue!(stdout, MoveTo(x, y + j as u16), Print(line))?;
    }
    Ok(())
}

// --- Main render entry point ---

pub fn redraw(stdout: &mut BufWriter<std::io::Stdout>, app: &mut AppState) -> std::io::Result<()> {
    let term_size = crossterm::terminal::size()?;
    begin_sync(stdout)?;

    for i in 0..app.instances.len() {
        let is_active = i == app.active;
        let dim = !is_active || app.confirm_remove;
        if app.instances[i].dirty || app.confirm_remove {
            app.instances[i].draw(stdout, is_active, dim, term_size)?;
            app.instances[i].dirty = false;
        }
    }

    if app.confirm_remove {
        display_remove_confirm(stdout, app.active, term_size)?;
    }

    end_sync(stdout)?;
    stdout.flush()
}
