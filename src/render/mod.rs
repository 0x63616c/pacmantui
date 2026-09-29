//! Kitty-graphics renderer and terminal lifecycle.
//!
//! Wire protocol and frame-loop shape: docs/research/rendering.md §4 and the
//! validated prototype (src/bin/proto.rs). Requirements: opaque #000000
//! everywhere, self integer-scaling (cap 4x), o=z, q=2, double-buffered ids,
//! sync-output brackets, cleanup on drop AND panic, no diagnostics on screen.
//!
//! Owner: W1-RENDER agent. Public signatures are the contract; extend, don't break.
//!
//! Internals: the seam is [`compose`] — the pure-CPU [`compose::Compositor`]
//! produces every finished frame of pixels at the logical resolution (map
//! tiles x 8px) with geometry owned by [`layout::FrameLayout`], and tests
//! exercise it through that same public interface. [`Renderer`] is the kitty
//! tty adapter on that seam: it integer-upscales the composed frame and
//! sends it as ONE image per frame (transmit new id, place z=-1, delete old
//! id, inside CSI 2026 brackets — see `kitty`); `terminal`/`kitty` own the
//! escape-code surface, unit-tested at their internal seams (injectable byte
//! sinks and pure layout math).

pub mod compose;
mod font;
mod framebuffer;
mod kitty;
pub mod layout;
mod sprites;
mod terminal;

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute};

use crate::map::Map;
use crate::types::RenderState;

pub use compose::Overlay;
use layout::FrameLayout;

/// Double-buffered kitty image ids (arbitrary; we own the tty's id space).
const ID_A: u32 = 42;
const ID_B: u32 = 43;

/// Logical input events surfaced to the app (decoupled from crossterm types).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Pause,
    Quit,
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    Key(Key),
    /// Terminal geometry changed; renderer has already recomputed its layout.
    Resized,
}

/// Data for menu-family screens (drawn by render, owned/decided by app).
#[derive(Debug, Clone)]
pub struct MenuScreen {
    pub title: String,
    pub items: Vec<MenuItem>,
    pub selected: usize,
    /// Footer/help line.
    pub footer: String,
    /// Optional decorative marquee (ghost parade etc.) toggle.
    pub decorated: bool,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    /// Right-aligned value column (e.g. current difficulty), if any.
    pub value: Option<String>,
    pub enabled: bool,
}

/// Loading/progress screen data (reflects real initialization work).
#[derive(Debug, Clone)]
pub struct LoadingScreen {
    pub message: String,
    /// 0.0..=1.0 actual progress.
    pub progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneKind {
    None,
    Game,
    Menu,
    Loading,
}

/// The kitty tty adapter over [`compose::Compositor`]: composes each scene
/// through the pure compositor, then letterboxes, integer-scales and
/// transmits the finished frame to the terminal.
pub struct Renderer {
    _guard: terminal::TermGuard,
    layout: terminal::Layout,
    /// Current logical (unscaled) resolution.
    logical: (usize, usize),
    comp: compose::Compositor,
    scaled: Vec<u8>,
    out: Vec<u8>,
    /// (next transmit id, id to delete) — swapped every frame.
    ids: (u32, u32),
    scene: SceneKind,
}

impl Renderer {
    /// Enter raw mode + alt screen, probe kitty support, paint black.
    /// Fails with a clear message if the terminal lacks kitty graphics or
    /// pixel-size reporting (also detects tmux/screen and refuses).
    pub fn new() -> io::Result<Renderer> {
        terminal::reject_multiplexer()?;
        let guard = terminal::TermGuard::acquire()?;
        // Probe before touching the alt screen; on error the guard restores.
        terminal::probe_kitty()?;
        execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            cursor::Hide
        )?;
        let layout = terminal::compute_layout(compose::MENU_W, compose::MENU_H)?;
        terminal::paint_black(&layout)?;
        Ok(Renderer {
            _guard: guard,
            layout,
            logical: (compose::MENU_W, compose::MENU_H),
            comp: compose::Compositor::new(),
            scaled: Vec::new(),
            out: Vec::new(),
            ids: (ID_A, ID_B),
            scene: SceneKind::None,
        })
    }

    /// Draw one gameplay frame for `state` over `map`, with the app-owned
    /// [`Overlay`] flags travelling as frame data (no sticky renderer
    /// state). The frame is the map grid plus any HUD padding rows (owned by
    /// [`FrameLayout`]) so the score/lives HUD never overprints an all-maze
    /// grid; for maps that embed their own dead rows (classic) the padding
    /// is zero and the frame equals the map grid.
    pub fn render_game(
        &mut self,
        map: &Map,
        state: &RenderState,
        overlay: Overlay,
    ) -> io::Result<()> {
        // FrameLayout owns the frame geometry; the compositor re-derives the
        // same layout internally (an O(tiles) scan, negligible per frame).
        let (lw, lh) = FrameLayout::of_map(map).frame_px();
        self.ensure_scene(SceneKind::Game, lw, lh)?;
        let frame = self.comp.game(map, state, overlay);
        Self::transmit(
            frame,
            &self.layout,
            &mut self.ids,
            &mut self.scaled,
            &mut self.out,
        )
    }

    pub fn render_menu(&mut self, screen: &MenuScreen) -> io::Result<()> {
        self.ensure_scene(SceneKind::Menu, compose::MENU_W, compose::MENU_H)?;
        let frame = self.comp.menu(screen);
        Self::transmit(
            frame,
            &self.layout,
            &mut self.ids,
            &mut self.scaled,
            &mut self.out,
        )
    }

    pub fn render_loading(&mut self, screen: &LoadingScreen) -> io::Result<()> {
        self.ensure_scene(SceneKind::Loading, compose::MENU_W, compose::MENU_H)?;
        let frame = self.comp.loading(screen);
        Self::transmit(
            frame,
            &self.layout,
            &mut self.ids,
            &mut self.scaled,
            &mut self.out,
        )
    }

    /// Poll input/resize events, waiting up to `timeout`.
    pub fn poll_events(&mut self, timeout: Duration) -> io::Result<Vec<AppEvent>> {
        let mut events = Vec::new();
        let deadline = Instant::now() + timeout;
        loop {
            let wait = if events.is_empty() {
                deadline.saturating_duration_since(Instant::now())
            } else {
                Duration::ZERO
            };
            if !crossterm::event::poll(wait)? {
                break;
            }
            match crossterm::event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    if let Some(key) = map_key(k.code, k.modifiers) {
                        events.push(AppEvent::Key(key));
                    }
                }
                Event::Resize(..) => {
                    self.handle_resize()?;
                    if !events.contains(&AppEvent::Resized) {
                        events.push(AppEvent::Resized);
                    }
                }
                _ => {}
            }
        }
        Ok(events)
    }

    /// Minimum terminal size check for the given map at 1x; returns a
    /// human-readable requirement string when too small. The requirement is
    /// owned by [`FrameLayout`], the same geometry `render_game` composes to.
    pub fn size_check(&self, map: &Map) -> Result<(), String> {
        let (need_w, need_h) = FrameLayout::of_map(map).min_terminal_px();
        if self.layout.px_w >= need_w && self.layout.px_h >= need_h {
            return Ok(());
        }
        let need_cols = need_w.div_ceil(self.layout.cell_w);
        let need_rows = need_h.div_ceil(self.layout.cell_h);
        Err(format!(
            "terminal window too small: this map needs {need_w}x{need_h} pixels at \
             1x scale (about {need_cols}x{need_rows} cells at the current cell size), \
             but the window is {}x{} pixels ({}x{} cells). Enlarge the window or \
             reduce the font size.",
            self.layout.px_w, self.layout.px_h, self.layout.cols, self.layout.rows
        ))
    }

    /// Switch scene / logical resolution; repaints the terminal black on any
    /// scene change or resolution change (and recomputes the layout). The
    /// compositor sizes its own frame; this only adapts the tty side.
    fn ensure_scene(&mut self, scene: SceneKind, lw: usize, lh: usize) -> io::Result<()> {
        if self.scene == scene && self.logical == (lw, lh) {
            return Ok(());
        }
        self.scene = scene;
        self.logical = (lw, lh);
        self.layout = terminal::compute_layout(lw, lh)?;
        terminal::paint_black(&self.layout)
    }

    fn handle_resize(&mut self) -> io::Result<()> {
        self.layout = terminal::compute_layout(self.logical.0, self.logical.1)?;
        terminal::paint_black(&self.layout)
    }

    /// The tty half of the adapter: scale the composed frame, assemble the
    /// kitty escape stream, write it in a single syscall, and swap the
    /// double-buffered ids. Takes fields (not `&mut self`) so the frame can
    /// stay borrowed from the compositor.
    fn transmit(
        frame: &compose::Frame,
        layout: &terminal::Layout,
        ids: &mut (u32, u32),
        scaled: &mut Vec<u8>,
        out: &mut Vec<u8>,
    ) -> io::Result<()> {
        framebuffer::scale_up(
            frame.data(),
            frame.width(),
            frame.height(),
            layout.scale,
            scaled,
        );
        let params = kitty::FrameParams {
            width: frame.width() * layout.scale,
            height: frame.height() * layout.scale,
            row: layout.anchor_row,
            col: layout.anchor_col,
            new_id: ids.0,
            old_id: ids.1,
            compress: true,
        };
        kitty::write_frame(out, scaled, &params);
        {
            let mut stdout = io::stdout().lock();
            stdout.write_all(out)?;
            stdout.flush()?;
        }
        *ids = (ids.1, ids.0);
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // Idempotent terminal restore + image deletion (the guard's drop and
        // the panic hook call the same function; whoever runs first wins).
        terminal::restore_terminal();
    }
}

fn map_key(code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
    Some(match code {
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Escape,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => Key::Quit,
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'w' => Key::Up,
            'a' => Key::Left,
            's' => Key::Down,
            'd' => Key::Right,
            'p' => Key::Pause,
            'q' => Key::Quit,
            other => Key::Char(other),
        },
        _ => return None,
    })
}
