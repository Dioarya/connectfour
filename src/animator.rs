use std::{
    io::Write,
    time::{Duration, Instant},
};

use crate::disk::Disk;
use crate::textures::Textures;
use crate::translate::Translate;

// Drives the falling-disk animation for a single placed disk.
// Uses analytical integration — position and velocity are computed exactly
// from elapsed time, with no accumulated numerical error regardless of framerate.
pub struct Animator {
    // --- Identity: which cell this animator represents ---
    pub disk: Disk,
    pub col: usize,
    pub row: usize,

    // --- Analytical physics state ---
    // y0 and v0 describe the current bounce segment; reset on each bounce.
    y0: f64,
    v0: f64,
    segment_start: Instant, // wall-clock time when the current segment began
    pub target_y: f64,
    pub gravity: f64,
    pub coeff_restitution: f64,
    pub settle_threshold: f64,

    // --- Timing ---
    pub start: Instant,
    pub timeout: Duration,

    // --- Dirty tracking ---
    last_displayed_y: i32,
}

impl Animator {
    pub fn new(disk: Disk, col: usize, row: usize, start_y: f64, target_y: f64) -> Self {
        let cfg = crate::config::Config::get();
        let now = Instant::now();
        Self {
            disk,
            col,
            row,
            y0: start_y,
            v0: 0.0,
            segment_start: now,
            target_y,
            gravity: cfg.gravity,
            coeff_restitution: cfg.coeff_restitution,
            settle_threshold: cfg.settle_threshold,
            start: now,
            timeout: Duration::MAX,
            last_displayed_y: start_y as i32,
        }
    }

    pub fn is_done(&self) -> bool {
        self.start.elapsed() >= self.timeout
    }

    // Exact position at time t into the current segment.
    fn y_at(&self, t: f64) -> f64 {
        (0.5 * self.gravity * t).mul_add(t, self.v0.mul_add(t, self.y0))
    }

    // Exact velocity at time t into the current segment.
    const fn v_at(&self, t: f64) -> f64 {
        self.gravity.mul_add(t, self.v0)
    }

    // Solves y0 + v0*t + 0.5*g*t^2 = target_y for the smallest positive t.
    fn time_to_bounce(&self) -> Option<f64> {
        let quad_a = 0.5 * self.gravity;
        let quad_b = self.v0;
        let quad_c = self.y0 - self.target_y;
        let discriminant = quad_b.mul_add(quad_b, -(4.0 * quad_a * quad_c));
        if discriminant < 0.0 {
            return None;
        }
        let sqrt_discriminant = discriminant.sqrt();
        let root1 = (-quad_b - sqrt_discriminant) / (2.0 * quad_a);
        let root2 = (-quad_b + sqrt_discriminant) / (2.0 * quad_a);
        match (root1 > 1e-9, root2 > 1e-9) {
            (true, true) => Some(root1.min(root2)),
            (true, false) => Some(root1),
            (false, true) => Some(root2),
            _ => None,
        }
    }

    // Advances the simulation to the current wall-clock time, resolving all
    // bounce events analytically. Returns true if the integer terminal row changed.
    pub fn update(&mut self) -> bool {
        if self.is_done() {
            return false;
        }

        loop {
            let t_now = self.segment_start.elapsed().as_secs_f64();

            if let Some(t_bounce) = self.time_to_bounce()
                && t_bounce <= t_now
            {
                let v_reflected = -self.v_at(t_bounce) * self.coeff_restitution;
                if v_reflected.abs() < self.settle_threshold {
                    self.y0 = self.target_y;
                    self.v0 = 0.0;
                    self.timeout = self.start.elapsed();
                    break;
                }
                self.segment_start += Duration::from_secs_f64(t_bounce);
                self.y0 = self.target_y;
                self.v0 = v_reflected;
                continue;
            }
            break;
        }

        let y_now = self
            .y_at(self.segment_start.elapsed().as_secs_f64())
            .min(self.target_y);
        let new_y = y_now as i32;
        if new_y == self.last_displayed_y {
            false
        } else {
            self.last_displayed_y = new_y;
            true
        }
    }

    pub fn current_y(&self) -> f64 {
        self.y_at(self.segment_start.elapsed().as_secs_f64())
            .min(self.target_y)
    }

    pub fn display(
        &self,
        stdout: &mut impl Write,
        textures: &Textures,
        translate: &Translate,
    ) -> std::io::Result<()> {
        let char_x = textures.char_x_positions[self.col];
        let char_y = self.current_y() as i32;
        self.disk
            .display(stdout, char_x, char_y, textures, translate)
    }
}
