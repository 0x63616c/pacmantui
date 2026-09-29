//! Rendering-architecture prototype (validation tool, not the game).
//!
//! Proves in a real Ghostty pane: animated kitty-graphics pixel output at a
//! fixed tick rate, responsive keyboard input, opaque black background,
//! resize handling, and clean exit — per docs/research/rendering.md §4.
//!
//! Controls: arrows steer, `p` pause, `q`/Esc/Ctrl-C quit.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, event, execute, terminal};

const LOGICAL_W: usize = 224;
const LOGICAL_H: usize = 288;
const TICK_HZ: f64 = 60.0;
const ID_A: u32 = 42;
const ID_B: u32 = 43;

struct TermGuard;

impl TermGuard {
    fn cleanup() {
        let mut out = io::stdout();
        // Delete all kitty images+data, end any sync update, restore terminal.
        let _ = out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\\x1b[?2026l\x1b[0m");
        let _ = execute!(out, cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        Self::cleanup();
    }
}

struct Layout {
    scale: usize,
    anchor_row: u16,
    anchor_col: u16,
    cols: u16,
    rows: u16,
}

fn compute_layout() -> io::Result<Layout> {
    let ws = terminal::window_size()?;
    if ws.width == 0 || ws.height == 0 {
        return Err(io::Error::other(
            "terminal does not report pixel size (needed for kitty graphics)",
        ));
    }
    let cell_w = ws.width as usize / ws.columns.max(1) as usize;
    let cell_h = ws.height as usize / ws.rows.max(1) as usize;
    let scale = (ws.width as usize / LOGICAL_W)
        .min(ws.height as usize / LOGICAL_H)
        .clamp(1, 4);
    let off_x = (ws.width as usize).saturating_sub(LOGICAL_W * scale) / 2;
    let off_y = (ws.height as usize).saturating_sub(LOGICAL_H * scale) / 2;
    Ok(Layout {
        scale,
        anchor_row: (off_y / cell_h.max(1)) as u16 + 1,
        anchor_col: (off_x / cell_w.max(1)) as u16 + 1,
        cols: ws.columns,
        rows: ws.rows,
    })
}

struct Ball {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    phase: u32,
}

fn compose(fb: &mut [u8], ball: &Ball) {
    fb.fill(0); // opaque black
    let put = |fb: &mut [u8], x: usize, y: usize, rgb: [u8; 3]| {
        let i = (y * LOGICAL_W + x) * 3;
        fb[i..i + 3].copy_from_slice(&rgb);
    };
    let blue = [33, 33, 222];
    // border walls, 2px
    for x in 0..LOGICAL_W {
        for y in [0, 1, LOGICAL_H - 2, LOGICAL_H - 1] {
            put(fb, x, y, blue);
        }
    }
    for y in 0..LOGICAL_H {
        for x in [0, 1, LOGICAL_W - 2, LOGICAL_W - 1] {
            put(fb, x, y, blue);
        }
    }
    // dot grid (pellet-ish)
    for ty in 1..(LOGICAL_H / 8 - 1) {
        for tx in 1..(LOGICAL_W / 8 - 1) {
            if (tx + ty) % 3 == 0 {
                let (cx, cy) = (tx * 8 + 3, ty * 8 + 3);
                for dy in 0..2 {
                    for dx in 0..2 {
                        put(fb, cx + dx, cy + dy, [255, 183, 174]);
                    }
                }
            }
        }
    }
    // pac-ish disc with animated mouth toward movement direction
    let (bx, by, r) = (ball.x, ball.y, 7.0f32);
    let mouth_open = (ball.phase / 6).is_multiple_of(2);
    let (mdx, mdy) = if ball.dx.abs() > ball.dy.abs() {
        (ball.dx.signum(), 0.0)
    } else {
        (0.0, ball.dy.signum())
    };
    for py in (by - r) as isize..=(by + r) as isize {
        for px in (bx - r) as isize..=(bx + r) as isize {
            if px < 0 || py < 0 || px >= LOGICAL_W as isize || py >= LOGICAL_H as isize {
                continue;
            }
            let (vx, vy) = (px as f32 - bx, py as f32 - by);
            if vx * vx + vy * vy > r * r {
                continue;
            }
            if mouth_open {
                // wedge: angle to movement dir within ~45°
                let dot = vx * mdx + vy * mdy;
                let len = (vx * vx + vy * vy).sqrt();
                if len > 0.5 && dot / len > 0.6 {
                    continue;
                }
            }
            put(fb, px as usize, py as usize, [255, 255, 0]);
        }
    }
}

fn scale_up(src: &[u8], dst: &mut Vec<u8>, k: usize) {
    dst.clear();
    dst.reserve(src.len() * k * k);
    let row_bytes = LOGICAL_W * 3 * k;
    for y in 0..LOGICAL_H {
        let row_start = dst.len();
        let src_row = &src[y * LOGICAL_W * 3..(y + 1) * LOGICAL_W * 3];
        for x in 0..LOGICAL_W {
            let px = &src_row[x * 3..x * 3 + 3];
            for _ in 0..k {
                dst.extend_from_slice(px);
            }
        }
        for _ in 1..k {
            dst.extend_from_within(row_start..row_start + row_bytes);
        }
    }
}

fn emit_frame(
    out: &mut Vec<u8>,
    scaled: &[u8],
    layout: &Layout,
    new_id: u32,
    old_id: u32,
    hud: &str,
) -> usize {
    out.clear();
    out.extend_from_slice(b"\x1b[?2026h");
    // HUD text (top-left, white on black)
    out.extend_from_slice(b"\x1b[1;1H\x1b[38;2;255;255;255m\x1b[48;2;0;0;0m");
    out.extend_from_slice(hud.as_bytes());
    out.extend_from_slice(b"\x1b[K");
    // cursor to anchor
    out.extend_from_slice(format!("\x1b[{};{}H", layout.anchor_row, layout.anchor_col).as_bytes());
    // zlib-compress (kitty o=z), then transmit f=24 in 4096-byte b64 chunks
    let mut enc = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(64 * 1024),
        flate2::Compression::fast(),
    );
    enc.write_all(scaled).expect("zlib compress");
    let compressed = enc.finish().expect("zlib finish");
    let payload_len = compressed.len();
    let b64 = B64.encode(&compressed);
    let bytes = b64.as_bytes();
    let (w, h) = (LOGICAL_W * layout.scale, LOGICAL_H * layout.scale);
    let mut first = true;
    let mut idx = 0;
    while idx < bytes.len() {
        let end = (idx + 4096).min(bytes.len());
        let more = if end < bytes.len() { 1 } else { 0 };
        if first {
            out.extend_from_slice(
                format!("\x1b_Ga=t,f=24,o=z,s={w},v={h},i={new_id},q=2,m={more};").as_bytes(),
            );
            first = false;
        } else {
            out.extend_from_slice(format!("\x1b_Gm={more};").as_bytes());
        }
        out.extend_from_slice(&bytes[idx..end]);
        out.extend_from_slice(b"\x1b\\");
        idx = end;
    }
    // place new under text, delete old placement
    out.extend_from_slice(format!("\x1b_Ga=p,i={new_id},p=1,z=-1,C=1,q=2\x1b\\").as_bytes());
    out.extend_from_slice(format!("\x1b_Ga=d,d=i,i={old_id},q=2\x1b\\").as_bytes());
    out.extend_from_slice(b"\x1b[?2026l");
    payload_len
}

fn paint_black(layout: &Layout) -> io::Result<()> {
    let mut out = io::stdout();
    // Explicit opaque black over every cell (2J with bg set), images deleted first.
    out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\\x1b[48;2;0;0;0m\x1b[2J")?;
    let blank = " ".repeat(layout.cols as usize);
    for row in 1..=layout.rows {
        out.write_all(format!("\x1b[{row};1H{blank}").as_bytes())?;
    }
    out.flush()
}

fn main() -> io::Result<()> {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        TermGuard::cleanup();
        default_hook(info);
    }));

    terminal::enable_raw_mode()?;
    let _guard = TermGuard;
    execute!(io::stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

    let mut layout = compute_layout()?;
    paint_black(&layout)?;

    let mut fb = vec![0u8; LOGICAL_W * LOGICAL_H * 3];
    let mut scaled: Vec<u8> = Vec::new();
    let mut frame_buf: Vec<u8> = Vec::new();

    let mut ball = Ball {
        x: 112.0,
        y: 220.0,
        dx: 1.26,
        dy: 0.0,
        phase: 0,
    };
    let mut paused = false;
    let mut frame: u64 = 0;
    let mut ids = (ID_A, ID_B);
    let tick = Duration::from_secs_f64(1.0 / TICK_HZ);
    let mut next_tick = Instant::now() + tick;
    let mut times: Vec<f64> = Vec::with_capacity(120);
    let (mut avg_ms, mut max_ms) = (0.0f64, 0.0f64);
    let mut last_payload: usize = 0;

    loop {
        // input until next tick
        while event::poll(next_tick.saturating_duration_since(Instant::now()))? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('p') => paused = !paused,
                    KeyCode::Up => (ball.dx, ball.dy) = (0.0, -1.26),
                    KeyCode::Down => (ball.dx, ball.dy) = (0.0, 1.26),
                    KeyCode::Left => (ball.dx, ball.dy) = (-1.26, 0.0),
                    KeyCode::Right => (ball.dx, ball.dy) = (1.26, 0.0),
                    _ => {}
                },
                Event::Resize(..) => {
                    layout = compute_layout()?;
                    paint_black(&layout)?;
                }
                _ => {}
            }
        }
        next_tick += tick;

        if !paused {
            ball.x += ball.dx;
            ball.y += ball.dy;
            ball.phase += 1;
            let r = 9.0;
            if ball.x < r || ball.x > LOGICAL_W as f32 - r {
                ball.dx = -ball.dx;
                ball.x = ball.x.clamp(r, LOGICAL_W as f32 - r);
            }
            if ball.y < r || ball.y > LOGICAL_H as f32 - r {
                ball.dy = -ball.dy;
                ball.y = ball.y.clamp(r, LOGICAL_H as f32 - r);
            }
        }

        let t0 = Instant::now();
        compose(&mut fb, &ball);
        scale_up(&fb, &mut scaled, layout.scale);
        let hud = format!(
            " proto {}x | frame {} | draw avg {:.2}ms max {:.2}ms | {:.0} KB/frame | arrows steer, p pause, q quit ",
            layout.scale,
            frame,
            avg_ms,
            max_ms,
            last_payload as f64 / 1024.0
        );
        last_payload = emit_frame(&mut frame_buf, &scaled, &layout, ids.0, ids.1, &hud);
        {
            let mut out = io::stdout().lock();
            out.write_all(&frame_buf)?;
            out.flush()?;
        }
        ids = (ids.1, ids.0);
        frame += 1;

        times.push(t0.elapsed().as_secs_f64() * 1000.0);
        if times.len() >= 60 {
            avg_ms = times.iter().sum::<f64>() / times.len() as f64;
            max_ms = times.iter().cloned().fold(0.0, f64::max);
            times.clear();
        }
    }
}
