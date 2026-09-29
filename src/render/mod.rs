//! Kitty-graphics renderer and terminal lifecycle.
//!
//! Wire protocol and frame-loop shape: docs/research/rendering.md §4 and the
//! validated prototype (src/bin/proto.rs). Requirements: opaque #000000
//! everywhere, self integer-scaling (cap 4x), o=z, q=2, double-buffered ids,
//! sync-output brackets, cleanup on drop AND panic, no diagnostics on screen.
//!
//! Owner: W1-RENDER agent. Public signatures are the contract; extend, don't break.

use std::io;
use std::time::Duration;

use crate::map::Map;
use crate::types::RenderState;

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

pub struct Renderer {
    // W1-RENDER defines internals.
    _private: (),
}

impl Renderer {
    /// Enter raw mode + alt screen, probe kitty support, paint black.
    /// Fails with a clear message if the terminal lacks kitty graphics or
    /// pixel-size reporting (also detects tmux/screen and refuses).
    pub fn new() -> io::Result<Renderer> {
        todo!("W1-RENDER")
    }

    /// Draw one gameplay frame for `state` over `map`.
    pub fn render_game(&mut self, map: &Map, state: &RenderState) -> io::Result<()> {
        let _ = (map, state);
        todo!("W1-RENDER")
    }

    pub fn render_menu(&mut self, screen: &MenuScreen) -> io::Result<()> {
        let _ = screen;
        todo!("W1-RENDER")
    }

    pub fn render_loading(&mut self, screen: &LoadingScreen) -> io::Result<()> {
        let _ = screen;
        todo!("W1-RENDER")
    }

    /// Poll input/resize events, waiting up to `timeout`.
    pub fn poll_events(&mut self, timeout: Duration) -> io::Result<Vec<AppEvent>> {
        let _ = timeout;
        todo!("W1-RENDER")
    }

    /// Minimum terminal size check for the given map at 1x; returns a
    /// human-readable requirement string when too small.
    pub fn size_check(&self, map: &Map) -> Result<(), String> {
        let _ = map;
        todo!("W1-RENDER")
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // W1-RENDER: idempotent terminal restore + image deletion.
    }
}
