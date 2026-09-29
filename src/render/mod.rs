//! Kitty-graphics renderer and terminal lifecycle.
//!
//! Wire protocol and frame-loop shape: docs/research/rendering.md §4 and the
//! validated prototype (src/bin/proto.rs). Requirements: opaque #000000
//! everywhere, self integer-scaling (cap 4x), o=z, q=2, double-buffered ids,
//! sync-output brackets, cleanup on drop AND panic, no diagnostics on screen.
//!
//! Owner: W1-RENDER agent. Public signatures are the contract; extend, don't break.
//!
//! Internals: everything is composed into a CPU framebuffer at the logical
//! resolution (map tiles x 8px), integer-upscaled, and sent as ONE image per
//! frame (transmit new id, place z=-1, delete old id, inside CSI 2026
//! brackets — see `kitty`). Submodules `framebuffer`/`sprites`/`font`/`scenes`
//! are pure CPU and unit-tested via [`test_api`]; `terminal`/`kitty` own the
//! escape-code surface.

mod font;
mod framebuffer;
mod kitty;
mod scenes;
mod sprites;
mod terminal;

use std::io::{self, Write};
use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::{cursor, execute};

use crate::map::{Cell, Map};
use crate::types::{RenderState, Sequence, TILE_PX, TilePos};

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

pub struct Renderer {
    _guard: terminal::TermGuard,
    layout: terminal::Layout,
    /// Current logical (unscaled) resolution.
    logical: (usize, usize),
    fb: framebuffer::Frame,
    scaled: Vec<u8>,
    out: Vec<u8>,
    /// (next transmit id, id to delete) — swapped every frame.
    ids: (u32, u32),
    scene: SceneKind,
    /// Static maze layer cache, keyed by map id: (blue, white-flash) variants.
    maze_key: Option<String>,
    maze_blue: Vec<u8>,
    maze_white: Vec<u8>,
    paused: bool,
    game_over: bool,
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
        let layout = terminal::compute_layout(scenes::MENU_W, scenes::MENU_H)?;
        terminal::paint_black(&layout)?;
        Ok(Renderer {
            _guard: guard,
            layout,
            logical: (scenes::MENU_W, scenes::MENU_H),
            fb: framebuffer::Frame::new(scenes::MENU_W, scenes::MENU_H),
            scaled: Vec::new(),
            out: Vec::new(),
            ids: (ID_A, ID_B),
            scene: SceneKind::None,
            maze_key: None,
            maze_blue: Vec::new(),
            maze_white: Vec::new(),
            paused: false,
            game_over: false,
        })
    }

    /// Show/hide the "PAUSED" overlay on subsequent gameplay frames.
    /// (Added for the app: pause state lives outside `RenderState`.)
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Show/hide the "GAME  OVER" overlay on subsequent gameplay frames.
    /// (Added for the app: `Sequence` has no game-over variant.)
    pub fn set_game_over(&mut self, game_over: bool) {
        self.game_over = game_over;
    }

    /// Draw one gameplay frame for `state` over `map`.
    pub fn render_game(&mut self, map: &Map, state: &RenderState) -> io::Result<()> {
        let (tw, th) = (map.width(), map.height());
        let lw = (tw * TILE_PX) as usize;
        let lh = (th * TILE_PX) as usize;
        self.ensure_scene(SceneKind::Game, lw, lh)?;

        let kind_at = |x: i32, y: i32| cell_kind(map, x, y);
        if self.maze_key.as_deref() != Some(map.id()) {
            self.maze_blue = scenes::compose_maze_layer(tw, th, &kind_at, false);
            self.maze_white = scenes::compose_maze_layer(tw, th, &kind_at, true);
            self.maze_key = Some(map.id().to_string());
        }

        let white = matches!(state.sequence, Sequence::LevelFlash { tick }
            if scenes::flash_is_white(tick));
        let layer = if white {
            &self.maze_white
        } else {
            &self.maze_blue
        };
        let fruit = map.fruit_pos();
        let view = scenes::GameView {
            layer,
            tw,
            th,
            state,
            fruit_px: (fruit.x.px(), fruit.y.px()),
            paused: self.paused,
            game_over: self.game_over,
        };
        scenes::draw_game(&mut self.fb, &view, kind_at);
        self.flush()
    }

    pub fn render_menu(&mut self, screen: &MenuScreen) -> io::Result<()> {
        self.ensure_scene(SceneKind::Menu, scenes::MENU_W, scenes::MENU_H)?;
        scenes::compose_menu(&mut self.fb, screen);
        self.flush()
    }

    pub fn render_loading(&mut self, screen: &LoadingScreen) -> io::Result<()> {
        self.ensure_scene(SceneKind::Loading, scenes::MENU_W, scenes::MENU_H)?;
        scenes::compose_loading(&mut self.fb, screen);
        self.flush()
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
    /// human-readable requirement string when too small.
    pub fn size_check(&self, map: &Map) -> Result<(), String> {
        let need_w = (map.width() * TILE_PX) as usize;
        let need_h = (map.height() * TILE_PX) as usize;
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
    /// scene change or resolution change (and recomputes the layout).
    fn ensure_scene(&mut self, scene: SceneKind, lw: usize, lh: usize) -> io::Result<()> {
        if self.scene == scene && self.logical == (lw, lh) {
            return Ok(());
        }
        self.scene = scene;
        self.logical = (lw, lh);
        self.fb.resize(lw, lh);
        self.layout = terminal::compute_layout(lw, lh)?;
        terminal::paint_black(&self.layout)
    }

    fn handle_resize(&mut self) -> io::Result<()> {
        self.layout = terminal::compute_layout(self.logical.0, self.logical.1)?;
        terminal::paint_black(&self.layout)
    }

    /// Scale the framebuffer, assemble the kitty frame, write it in a single
    /// syscall, and swap the double-buffered ids.
    fn flush(&mut self) -> io::Result<()> {
        framebuffer::scale_up(
            self.fb.data(),
            self.fb.width(),
            self.fb.height(),
            self.layout.scale,
            &mut self.scaled,
        );
        let params = kitty::FrameParams {
            width: self.fb.width() * self.layout.scale,
            height: self.fb.height() * self.layout.scale,
            row: self.layout.anchor_row,
            col: self.layout.anchor_col,
            new_id: self.ids.0,
            old_id: self.ids.1,
            compress: true,
        };
        kitty::write_frame(&mut self.out, &self.scaled, &params);
        {
            let mut stdout = io::stdout().lock();
            stdout.write_all(&self.out)?;
            stdout.flush()?;
        }
        self.ids = (self.ids.1, self.ids.0);
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

fn cell_kind(map: &Map, x: i32, y: i32) -> scenes::TileKind {
    match map.cell(TilePos::new(x, y)) {
        Cell::Wall => scenes::TileKind::Wall,
        Cell::Door => scenes::TileKind::Door,
        Cell::Dot => scenes::TileKind::Dot,
        Cell::Energizer => scenes::TileKind::Energizer,
        Cell::Path | Cell::Tunnel | Cell::House => scenes::TileKind::Open,
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

/// Internal APIs re-exported for the crate's CPU-only render tests
/// (tests/render_*.rs). Not part of the public contract.
#[doc(hidden)]
pub mod test_api {
    pub use super::font::{
        GLYPH_W, draw_mini_number, draw_text, draw_text_scaled, glyph, mini_number_width,
        text_width,
    };
    pub use super::framebuffer::{Frame, scale_up};
    pub use super::kitty::{CHUNK, FrameParams, write_frame};
    pub use super::scenes::{
        GameView, MENU_H, MENU_W, TileKind, compose_loading, compose_maze_layer, compose_menu,
        draw_game, draw_hud, flash_is_white,
    };
    pub use super::sprites::{
        BODY, CYAN, DOOR_PINK, DOT_PEACH, ENERGIZER, EYES_D, EYES_L, EYES_R, EYES_U, FRIGHT_A,
        FRIGHT_B, FRIGHT_BLUE, FRIGHT_FLASH_REMAP, GHOST_A, GHOST_B, GREEN, IDENTITY, MAZE_BLUE,
        ORANGE, PAC_CLOSED, PAC_OPEN_D, PAC_OPEN_L, PAC_OPEN_R, PAC_OPEN_U, PALETTE, PINK, RED,
        Sprite, TAN, TRANSPARENT, WHITE, YELLOW, body_remap, eyes_sprite, fright_sprite,
        fruit_sprite, ghost_body, ghost_remap, life_sprite, pac_death_sprite, pac_sprite, rgb,
    };
    pub use super::terminal::{Layout, layout_for};
}
