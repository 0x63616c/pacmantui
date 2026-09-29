//! Deterministic game simulation (the arcade rules engine).
//!
//! Pure: no I/O, no clock, no terminal. One `tick` = one arcade frame.
//! Rule sources: docs/research/dossier-mechanics.md (+ tables via `rules`,
//! supplemental timings in `timings` with provenance).
//!
//! Owner: W1-SIM agent. Public signatures are the contract; extend, don't break.

pub mod timings;

use crate::map::Map;
use crate::rules::Rules;
use crate::types::{Difficulty, Event, InputFrame, RenderState};

#[derive(Debug, Clone)]
pub struct Game {
    // W1-SIM defines internals.
    _private: (),
}

impl Game {
    /// A fresh game on `map` at level 1 with `lives` from timings/defaults.
    pub fn new(map: Map, rules: Rules, difficulty: Difficulty, seed: u64) -> Game {
        let _ = (map, rules, difficulty, seed);
        todo!("W1-SIM")
    }

    /// Advance exactly one tick. Returns events emitted this tick, in order.
    pub fn tick(&mut self, input: InputFrame) -> Vec<Event> {
        let _ = input;
        todo!("W1-SIM")
    }

    /// Renderer snapshot for the current state.
    pub fn render_state(&self) -> RenderState {
        todo!("W1-SIM")
    }

    pub fn is_game_over(&self) -> bool {
        todo!("W1-SIM")
    }
    pub fn score(&self) -> u32 {
        todo!("W1-SIM")
    }
    pub fn level(&self) -> u32 {
        todo!("W1-SIM")
    }
    pub fn lives(&self) -> u8 {
        todo!("W1-SIM")
    }
    pub fn map(&self) -> &Map {
        todo!("W1-SIM")
    }
}

/// Deterministic 32-bit xorshift PRNG (documented; replays store the seed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(pub u32);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Fold the 64-bit seed; avoid the all-zero state.
        let s = (seed as u32) ^ ((seed >> 32) as u32) ^ 0x9E37_79B9;
        Rng(if s == 0 { 0x9E37_79B9 } else { s })
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
