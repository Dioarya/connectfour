use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::AppState;
use crate::config::Config;
use crate::disk::Disk;
use crate::render::{can_add_instance, update_all_translates};
use crate::runtime::Runtime;
use crate::types::{GameState, HistoryEntry, MoveRecord};

// Signals back to the main loop what kind of redraw (if any) is needed.
pub enum InputResult {
    Continue,  // nothing changed
    Quit,      // exit the program
    Redraw,    // only the active instance changed
    RedrawAll, // all instances affected (dim state, layout, active switch)
    Clear,     // full screen clear + redraw all
}

// --- Drop helper (shared by Enter/Space and 1-7) ---

fn do_drop(rt: &mut Runtime, col: usize) -> bool {
    rt.confirm_reset = false;
    rt.action_banner = None;

    let matches_redo = matches!(
        rt.redo_stack.last(),
        Some(HistoryEntry::Move(r)) if r.col == col && r.player == rt.current_player
    );
    if matches_redo {
        rt.redo_stack.pop();
    } else {
        rt.redo_stack.clear();
    }

    let (r, g, b) = Config::get().player_colors[rt.current_player];
    let disk = Disk {
        player: rt.current_player as u8,
        color: (r, g, b),
    };

    let Ok(animator) = rt.game.drop_animated(disk, col) else {
        return false;
    };

    let row = animator.row;
    rt.selected_col = col;
    rt.animators[col][row] = Some(animator);
    rt.history.push(HistoryEntry::Move(MoveRecord {
        col,
        row,
        player: rt.current_player,
    }));

    match rt.game.state {
        GameState::Won(p) => {
            rt.scores[p] += 1;
            rt.update_score_strings();
        }
        GameState::Playing => {
            rt.current_player = 1 - rt.current_player;
        }
        GameState::Tie => {}
    }
    rt.show_cursor = false;
    rt.dirty = true;
    true
}

fn key_matches(key: crossterm::event::KeyCode, binds: &[crossterm::event::KeyCode]) -> bool {
    binds.contains(&key)
}

// --- Key handler ---

pub fn handle_key(key: crossterm::event::KeyEvent, app: &mut AppState) -> InputResult {
    if key.kind != KeyEventKind::Press {
        return InputResult::Continue;
    }

    let binds = &Config::get().keybinds;

    // quit always works regardless of modal state
    if key_matches(key.code, &binds.quit) && key.modifiers.is_empty() {
        return InputResult::Quit;
    }

    // Ctrl+Shift+R — hard reset
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.modifiers.contains(KeyModifiers::SHIFT)
        && key.code == KeyCode::Char('R')
    {
        app.active_mut().hard_reset();
        return InputResult::Clear;
    }

    // Ctrl combos — instance navigation
    if key.modifiers == KeyModifiers::CONTROL {
        let instance_count = app.instances.len();
        match key.code {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Some(idx) = c.to_digit(10).map(|d| d as usize - 1)
                    && idx < instance_count
                {
                    app.active = idx;
                    return InputResult::RedrawAll;
                }
            }
            KeyCode::Char('a') | KeyCode::Left => {
                app.active = (app.active + instance_count - 1).rem_euclid(instance_count);
                return InputResult::RedrawAll;
            }
            KeyCode::Char('d') | KeyCode::Right => {
                app.active = (app.active + 1).rem_euclid(instance_count);
                return InputResult::RedrawAll;
            }
            _ => {}
        }
        return InputResult::Continue;
    }

    // Board-remove confirm dialog — swallows all keys except Y / N / Esc
    if app.confirm_remove {
        match key.code {
            KeyCode::Char('Y') => {
                if app.instances.len() > 1 {
                    app.instances.remove(app.active);
                    app.active = app.active.min(app.instances.len() - 1);
                    update_all_translates(&mut app.instances);
                }
                app.confirm_remove = false;
                return InputResult::Clear;
            }
            KeyCode::Char('N') | KeyCode::Esc => {
                app.confirm_remove = false;
                return InputResult::Clear;
            }
            _ => return InputResult::Continue,
        }
    }

    // Reset confirm dialog — swallows all keys except Y / N / Esc
    if app.active_mut().confirm_reset {
        match key.code {
            KeyCode::Char('Y') => {
                app.active_mut().reset_game();
                return InputResult::Clear;
            }
            KeyCode::Char('N') | KeyCode::Esc => {
                app.active_mut().confirm_reset = false;
                return InputResult::Clear;
            }
            _ => return InputResult::Continue,
        }
    }

    // Normal gameplay keys

    // --- Instance management ---
    if key.code == KeyCode::Char('+') && can_add_instance(&app.instances) {
        app.instances.push(Runtime::new());
        update_all_translates(&mut app.instances);
        app.active = app.instances.len() - 1;
        return InputResult::Clear;
    }
    if key.code == KeyCode::Char('-') && app.instances.len() > 1 {
        app.confirm_remove = true;
        return InputResult::RedrawAll;
    }

    // --- Reset ---
    if key_matches(key.code, &binds.reset) {
        let rt = app.active_mut();
        let has_pieces = rt.game.grid.col_fill.iter().any(|&f| f > 0);
        let game_over = rt.game.state != GameState::Playing;
        if game_over || !has_pieces {
            rt.reset_game();
            return InputResult::Clear;
        }
        rt.confirm_reset = true;
        return InputResult::Redraw;
    }

    // --- Column selection ---
    if key_matches(key.code, &binds.move_left) {
        let rt = app.active_mut();
        if rt.game.state == GameState::Playing {
            rt.selected_col =
                (rt.selected_col + rt.game.grid.width - 1).rem_euclid(rt.game.grid.width);
            rt.show_cursor = true;
            rt.dirty = true;
            return InputResult::Redraw;
        }
    }
    if key_matches(key.code, &binds.move_right) {
        let rt = app.active_mut();
        if rt.game.state == GameState::Playing {
            rt.selected_col = (rt.selected_col + 1).rem_euclid(rt.game.grid.width);
            rt.show_cursor = true;
            rt.dirty = true;
            return InputResult::Redraw;
        }
    }

    // --- Drop ---
    if key_matches(key.code, &binds.drop) {
        let rt = app.active_mut();
        if rt.game.state == GameState::Playing {
            let col = rt.selected_col;
            do_drop(rt, col);
            return InputResult::Redraw;
        }
    }

    // Number keys 1-7 always drop directly regardless of keybind config
    if let KeyCode::Char(c) = key.code
        && ('1'..='7').contains(&c)
    {
        let rt = app.active_mut();
        if rt.game.state == GameState::Playing {
            let col = (c as usize) - ('1' as usize);
            do_drop(rt, col);
            return InputResult::Redraw;
        }
    }

    // --- Undo / redo ---
    if key_matches(key.code, &binds.undo) {
        let rt = app.active_mut();
        if !rt.history.is_empty() {
            rt.undo();
            return InputResult::Redraw;
        }
    }
    if key_matches(key.code, &binds.redo) {
        let rt = app.active_mut();
        if !rt.redo_stack.is_empty() {
            rt.redo();
            return InputResult::Redraw;
        }
    }

    InputResult::Continue
}

// --- Event dispatcher ---

pub fn handle_event(app: &mut AppState) -> std::io::Result<InputResult> {
    match event::read()? {
        Event::Key(key) => Ok(handle_key(key, app)),
        Event::Resize(_, _) => {
            update_all_translates(&mut app.instances);
            Ok(InputResult::Clear)
        }
        _ => Ok(InputResult::Continue),
    }
}
