//! pacmantui — Pac-Man in the terminal with real pixel graphics (kitty protocol).
//!
//! Module contracts: see docs/plan/ARCHITECTURE.md and docs/plan/IMPLEMENTATION.md.
//! Dependency direction: `app` → `render`/`sim`/`replay`; `sim` → `rules`/`map`;
//! nothing below `app` touches the terminal or the clock.

pub mod app;
pub mod map;
pub mod render;
pub mod replay;
pub mod rules;
pub mod sim;
pub mod types;
