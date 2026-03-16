mod animator;
mod app;
mod config;
mod constants;
mod disk;
mod game;
mod grid;
mod input;
mod render;
mod runtime;
mod sparse;
mod strings;
mod textures;
mod translate;
mod types;

use std::{
    io::{BufWriter, Write},
    time::Instant,
};

use crossterm::{
    cursor::{Hide, Show},
    queue,
    terminal::{
        Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use app::AppState;
use config::Config;
use input::{InputResult, handle_event};
use render::redraw;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::get();
    let frame_duration = std::time::Duration::from_micros(1_000_000 / cfg.fps.max(1));

    let mut app = AppState::new();

    // Add any extra starting instances requested
    for _ in 1..cfg.starting_instances {
        app.instances.push(runtime::Runtime::new());
    }
    if cfg.starting_instances > 1 {
        render::update_all_translates(&mut app.instances);
    }

    let stdout = std::io::stdout();
    let mut stdout = BufWriter::new(stdout);

    enable_raw_mode()?;
    queue!(stdout, EnterAlternateScreen, Hide)?;
    stdout.flush()?;

    let mut last_frame = Instant::now();
    let mut needs_redraw = true;

    loop {
        let has_active_animators = app.instances.iter().any(|inst| {
            inst.animators.iter().flatten().any(|cell| cell.is_some())
        });

        let timeout = if cfg.vfr && !has_active_animators {
            std::time::Duration::MAX
        } else if cfg.vfr {
            std::time::Duration::ZERO
        } else {
            frame_duration.saturating_sub(last_frame.elapsed())
        };

        if crossterm::event::poll(timeout)? {
            match handle_event(&mut app)? {
                InputResult::Quit => break,
                InputResult::Clear => {
                    queue!(stdout, Clear(ClearType::All))?;
                    for inst in &mut app.instances {
                        inst.dirty = true;
                    }
                    needs_redraw = true;
                }
                InputResult::RedrawAll => {
                    for inst in &mut app.instances {
                        inst.dirty = true;
                    }
                    needs_redraw = true;
                }
                InputResult::Redraw => {
                    app.active_mut().dirty = true;
                    needs_redraw = true;
                }
                InputResult::Continue => {}
            }
        }

        // Animator tick — runs every frame in fixed-rate mode,
        // or on every loop iteration in VFR (immediately after each input event).
        let should_tick = if cfg.vfr {
            true
        } else {
            last_frame.elapsed() >= frame_duration
        };

        if should_tick {
            if !cfg.vfr {
                last_frame = Instant::now();
            }

            for inst in &mut app.instances {
                let mut visual_changed = false;
                let mut any_done = false;

                for col in &mut inst.animators {
                    for cell in col.iter_mut() {
                        if let Some(animator) = cell {
                            if animator.update() {
                                visual_changed = true;
                            }
                            if animator.is_done() {
                                *cell = None;
                                any_done = true;
                            }
                        }
                    }
                }

                if visual_changed || any_done {
                    inst.dirty = true;
                    needs_redraw = true;
                }
            }

            if needs_redraw {
                redraw(&mut stdout, &mut app)?;
                needs_redraw = false;
            }
        }
    }

    queue!(stdout, LeaveAlternateScreen, Show)?;
    stdout.flush()?;
    disable_raw_mode()?;
    Ok(())
}
