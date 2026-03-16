use std::io::Write;

use crossterm::{cursor::MoveRight, queue, style::Print};

// A pre-parsed segment of a sparse texture line.
// Computed once at init; rendered each frame without re-scanning the source string.
pub enum SparseSegment {
    Text(String),
    Skip(u16),
}

// Parses a line into a sequence of text runs and space-skips.
// Trailing spaces are dropped — MoveRight at end of line is pointless.
pub fn parse_sparse_line(line: &str) -> Vec<SparseSegment> {
    let mut segments = Vec::new();
    let mut text_buf = String::new();
    let mut skip_len: u16 = 0;

    for ch in line.chars() {
        if ch == ' ' {
            if !text_buf.is_empty() {
                segments.push(SparseSegment::Text(std::mem::take(&mut text_buf)));
            }
            skip_len += 1;
        } else {
            if skip_len > 0 {
                segments.push(SparseSegment::Skip(skip_len));
                skip_len = 0;
            }
            text_buf.push(ch);
        }
    }
    if !text_buf.is_empty() {
        segments.push(SparseSegment::Text(text_buf));
    }
    segments
}

// Renders a pre-parsed sparse line to the terminal.
pub fn render_sparse_line(
    stdout: &mut impl Write,
    segments: &[SparseSegment],
) -> std::io::Result<()> {
    for seg in segments {
        match seg {
            SparseSegment::Text(s) => queue!(stdout, Print(s))?,
            SparseSegment::Skip(n) => queue!(stdout, MoveRight(*n))?,
        }
    }
    Ok(())
}
