//! Reference tables and per-level parameter resolution (Dossier Appendix A).
//!
//! Data source of truth: docs/research/tables.md / tables.json. Constants here
//! must be transcribed from those docs; tests assert against values quoted
//! directly from the docs (never derived from these constants).
//!
//! Owner: W1-RULES agent. Public signatures are the contract; extend, don't break.

use crate::types::Difficulty;

/// One row of Table A.1, converted to implementation units.
/// Speeds are percent of full speed (see `types::FULL_SPEED_FIX8`); `None`
/// fright fields encode the source's "–" (ghosts never turn blue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSpec {
    pub bonus_symbol: BonusSymbol,
    pub bonus_points: u32,
    pub pac_speed_pct: u32,
    pub pac_dots_speed_pct: u32,
    pub ghost_speed_pct: u32,
    pub ghost_tunnel_speed_pct: u32,
    pub elroy1_dots_left: u32,
    pub elroy1_speed_pct: u32,
    pub elroy2_dots_left: u32,
    pub elroy2_speed_pct: u32,
    pub fright_pac_speed_pct: Option<u32>,
    pub fright_pac_dots_speed_pct: Option<u32>,
    pub fright_ghost_speed_pct: Option<u32>,
    /// Frightened duration in ticks; None ⇒ no frightened mode on this level
    /// (energizers still reverse ghosts).
    pub fright_ticks: Option<u32>,
    pub fright_flashes: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BonusSymbol {
    Cherries,
    Strawberry,
    Peach,
    Apple,
    Grapes,
    Galaxian,
    Bell,
    Key,
}

/// One phase of the scatter/chase alternation schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulePhase {
    pub mode: crate::types::Mode,
    /// Duration in ticks; `None` = indefinite (final chase).
    pub ticks: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Rules;

impl Rules {
    pub fn classic() -> Rules {
        Rules
    }

    /// Effective Table A.1 level for a played level under a difficulty
    /// (Hard maps via Table A.2's level elimination).
    pub fn effective_level(&self, level: u32, difficulty: Difficulty) -> u32 {
        let _ = (level, difficulty);
        todo!("W1-RULES")
    }

    /// Resolved spec for a played level (difficulty already applied).
    pub fn level_spec(&self, level: u32, difficulty: Difficulty) -> &LevelSpec {
        let _ = (level, difficulty);
        todo!("W1-RULES")
    }

    /// Scatter/chase schedule for a played level (difficulty applied).
    pub fn schedule(&self, level: u32, difficulty: Difficulty) -> &[SchedulePhase] {
        let _ = (level, difficulty);
        todo!("W1-RULES")
    }

    /// Per-ghost personal dot-counter limits [Blinky, Pinky, Inky, Clyde]
    /// for a played level (dossier Ch.4; classic values, pre map-scaling).
    pub fn house_dot_limits(&self, level: u32, difficulty: Difficulty) -> [u32; 4] {
        let _ = (level, difficulty);
        todo!("W1-RULES")
    }

    /// Global-counter release thresholds (after a death) per ghost.
    pub fn global_dot_limits(&self) -> [u32; 4] {
        todo!("W1-RULES")
    }

    /// Ticks without eating a dot before the next waiting ghost is released.
    pub fn no_dot_release_ticks(&self, level: u32, difficulty: Difficulty) -> u32 {
        let _ = (level, difficulty);
        todo!("W1-RULES")
    }

    /// Dots eaten at which fruit appears (classic: [70, 170]).
    pub fn fruit_trigger_dots(&self) -> [u32; 2] {
        todo!("W1-RULES")
    }

    /// Score awarded for eating the nth ghost of one energizer chain (1-based).
    pub fn ghost_chain_score(&self, chain: u8) -> u32 {
        let _ = chain;
        todo!("W1-RULES")
    }

    pub fn extra_life_score(&self) -> u32 {
        todo!("W1-RULES")
    }
}
