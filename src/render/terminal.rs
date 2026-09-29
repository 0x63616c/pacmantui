//! Terminal lifecycle: raw mode, alt screen, kitty capability probe,
//! tmux/screen refusal, layout math, opaque-black repaints, and idempotent
//! restore on drop AND panic (per docs/research/rendering.md §4.4 and the
//! validated prototype src/bin/proto.rs).

use std::io::{self, Read, Write};
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossterm::{cursor, execute, terminal};

/// True while the terminal is in game state and must be restored on exit.
static RESTORE_ACTIVE: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK: Once = Once::new();

/// Idempotent terminal restore: delete kitty images (placements AND data),
/// end any synchronized update, reset colors, show cursor, leave the alt
/// screen, disable raw mode. Safe to call any number of times.
pub fn restore_terminal() {
    if !RESTORE_ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let mut out = io::stdout();
    let _ = out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\\x1b[?2026l\x1b[0m");
    let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();
}

/// RAII guard: enables raw mode on acquire, restores the terminal on drop.
/// Also installs (once, chained) a panic hook that restores first so the
/// backtrace prints onto a working screen.
pub struct TermGuard {
    _priv: (),
}

impl TermGuard {
    pub fn acquire() -> io::Result<TermGuard> {
        terminal::enable_raw_mode()?;
        RESTORE_ACTIVE.store(true, Ordering::SeqCst);
        PANIC_HOOK.call_once(|| {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore_terminal();
                prev(info);
            }));
        });
        Ok(TermGuard { _priv: () })
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

/// Refuse to run under tmux/screen: they do not pass the kitty graphics
/// protocol through (rendering.md §4.6 risk 5).
pub fn reject_multiplexer() -> io::Result<()> {
    let term = std::env::var("TERM").unwrap_or_default();
    if std::env::var_os("TMUX").is_some() || term.starts_with("tmux") || term.starts_with("screen")
    {
        return Err(io::Error::other(
            "running inside tmux/screen, which does not pass the kitty graphics \
             protocol through. pacmantui needs real pixel graphics: run it directly \
             in Ghostty or kitty (no terminal multiplexer in between).",
        ));
    }
    Ok(())
}

/// Kitty graphics capability probe (rendering.md §1.6): a 1x1 `a=q` dummy
/// query followed by DA1 (`CSI c`). Every terminal answers DA1; only kitty
/// protocol terminals answer the graphics query first with `i=31;OK`. Must be
/// called with raw mode already enabled and before crossterm event polling
/// starts (it reads stdin directly).
pub fn probe_kitty() -> io::Result<()> {
    let mut out = io::stdout();
    out.write_all(b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c")?;
    out.flush()?;

    let mut stdin = io::stdin().lock();
    let mut buf: Vec<u8> = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match stdin.read(&mut byte) {
            Ok(0) => break, // EOF: not an interactive terminal
            Ok(_) => buf.push(byte[0]),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
        if da1_reply_complete(&buf) || buf.len() > 512 || Instant::now() > deadline {
            break;
        }
    }

    // Any `_Gi=31` graphics response before the DA1 reply — OK or error —
    // proves the terminal speaks the protocol (rendering.md §1.6); DA1-only
    // means it does not.
    if contains(&buf, b"_Gi=31") {
        Ok(())
    } else {
        let term = std::env::var("TERM").unwrap_or_default();
        Err(io::Error::other(format!(
            "this terminal did not answer the kitty graphics probe (TERM={term}). \
             pacmantui draws real pixels via the kitty graphics protocol and needs \
             a supporting terminal such as Ghostty or kitty.",
        )))
    }
}

/// True once the buffer holds a complete DA1 reply (`ESC [ ? ... c`).
fn da1_reply_complete(buf: &[u8]) -> bool {
    let mut i = 0;
    while i + 2 < buf.len() {
        if buf[i] == 0x1b && buf[i + 1] == b'[' && buf[i + 2] == b'?' {
            return buf[i + 3..].contains(&b'c');
        }
        i += 1;
    }
    false
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Screen geometry: integer scale, centered anchor cell, pixel/cell metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Integer upscale factor, 1..=4.
    pub scale: usize,
    /// 1-based cell of the top-left corner of the centered image.
    pub anchor_row: u16,
    pub anchor_col: u16,
    pub cols: u16,
    pub rows: u16,
    /// Terminal text area size in pixels.
    pub px_w: usize,
    pub px_h: usize,
    /// Cell size in pixels.
    pub cell_w: usize,
    pub cell_h: usize,
}

/// Pure layout math (unit-testable): pick the largest integer scale 1..=4
/// that fits, center the scaled image, and derive its anchor cell.
pub fn layout_for(
    px_w: usize,
    px_h: usize,
    cols: u16,
    rows: u16,
    logical_w: usize,
    logical_h: usize,
) -> Layout {
    let cell_w = (px_w / cols.max(1) as usize).max(1);
    let cell_h = (px_h / rows.max(1) as usize).max(1);
    let scale = (px_w / logical_w.max(1))
        .min(px_h / logical_h.max(1))
        .clamp(1, 4);
    let off_x = px_w.saturating_sub(logical_w * scale) / 2;
    let off_y = px_h.saturating_sub(logical_h * scale) / 2;
    Layout {
        scale,
        anchor_row: (off_y / cell_h) as u16 + 1,
        anchor_col: (off_x / cell_w) as u16 + 1,
        cols,
        rows,
        px_w,
        px_h,
        cell_w,
        cell_h,
    }
}

/// Query the terminal and compute the layout for a logical resolution.
/// Errors clearly when the terminal does not report pixel sizes.
pub fn compute_layout(logical_w: usize, logical_h: usize) -> io::Result<Layout> {
    let ws = terminal::window_size()?;
    if ws.width == 0 || ws.height == 0 {
        return Err(io::Error::other(
            "terminal does not report its pixel size (TIOCGWINSZ), which kitty \
             graphics placement needs. Use a terminal that does, such as Ghostty \
             or kitty.",
        ));
    }
    Ok(layout_for(
        ws.width as usize,
        ws.height as usize,
        ws.columns,
        ws.rows,
        logical_w,
        logical_h,
    ))
}

/// Paint the entire terminal opaque black (explicit SGR background + space
/// fill of every cell, like the prototype's paint_black), deleting all kitty
/// images first. Used at setup, on resize, and on scene changes.
pub fn paint_black(layout: &Layout) -> io::Result<()> {
    let mut buf: Vec<u8> =
        Vec::with_capacity((layout.cols as usize + 12) * layout.rows as usize + 64);
    buf.extend_from_slice(b"\x1b_Ga=d,d=A,q=2\x1b\\\x1b[48;2;0;0;0m\x1b[2J");
    let blank = " ".repeat(layout.cols as usize);
    for row in 1..=layout.rows {
        let _ = write!(buf, "\x1b[{row};1H{blank}");
    }
    buf.extend_from_slice(b"\x1b[0m");
    let mut out = io::stdout().lock();
    out.write_all(&buf)?;
    out.flush()
}
