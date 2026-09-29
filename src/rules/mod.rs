//! Reference tables and per-level parameter resolution (Dossier Appendix A).
//!
//! Data source of truth: docs/research/tables.md / tables.json. Constants here
//! must be transcribed from those docs; tests assert against values quoted
//! directly from the docs (never derived from these constants).
//!
//! Owner: W1-RULES agent. Public signatures are the contract; extend, don't break.
//!
//! # Time units
//!
//! One tick = one sim frame. The source tables give durations in seconds; the
//! arcade counts frames, so durations are stored as `round(seconds * 60)`
//! ticks (all source values are integral seconds, so the conversion is exact).
//! Interpretation note (tables.md; mechanics doc §8.3): the arcade's real
//! frame rate is 60.606061 Hz — Table A.1's title defines "100% speed =
//! 75.75757625 pixels/sec", i.e. 1.25 px/frame at 60.606061 Hz — so N table
//! "seconds" elapse in ~1% less wall-clock time than N seconds. The dossier's
//! own prose conflates "1/60th of a second" with one frame; we adopt that
//! frame-counting reading (60 ticks per table-second) instead of rescaling.

use crate::types::Difficulty;

mod tables;

/// One row of Table A.1, converted to implementation units.
/// Speeds are percent of full speed (see `types::FULL_SPEED_FIX8`); `None`
/// fright fields encode the source's "–" (ghosts never turn blue).
///
/// # The `~` dots-speed columns are informational, not movement inputs
///
/// The source marks every value of the "Pac-Man Dots Speed" columns with `~`:
/// they are *effective average* speeds while eating dots, produced by Pac-Man
/// halting 1 frame per dot (3 frames per energizer) on top of moving at the
/// plain speed columns (mechanics doc §2.4; tables.md "Units"). The sim must
/// therefore move Pac-Man at `pac_speed_pct` / `fright_pac_speed_pct` and
/// apply the eating pauses explicitly — using the dots columns as movement
/// speeds *as well* would double-count the slowdown. Mechanics doc §8.2 is
/// explicit: "Implement the stop frames, not the percentages." Both columns
/// are still exposed here because they are part of the transcribed table
/// (useful for display/verification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSpec {
    pub bonus_symbol: BonusSymbol,
    pub bonus_points: u32,
    /// Pac-Man movement speed (percent of full). THE speed input for the sim.
    pub pac_speed_pct: u32,
    /// Informational/derived (`~` in the source): average speed while eating
    /// dots. Do NOT use as a movement speed — see the struct docs.
    pub pac_dots_speed_pct: u32,
    pub ghost_speed_pct: u32,
    pub ghost_tunnel_speed_pct: u32,
    pub elroy1_dots_left: u32,
    pub elroy1_speed_pct: u32,
    pub elroy2_dots_left: u32,
    pub elroy2_speed_pct: u32,
    /// Pac-Man movement speed while ghosts are frightened.
    pub fright_pac_speed_pct: Option<u32>,
    /// Informational/derived (`~` in the source) — see `pac_dots_speed_pct`.
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
    ///
    /// Normal is the identity. Hard (Table A.2, tables.md): only levels
    /// 1, 3, 6, 19 and 20 are eliminated from play, so hard's Nth board is
    /// the Nth surviving gameplay level — 2, 4, 5, 7, 8, …, 18, then 21+.
    /// The table's bonus-ordinal columns confirm the board indexing: hard
    /// gameplay level 2 shows Cherries (symbol #1 ⇒ board 1) and hard
    /// gameplay 21+ shows "Key 4" (symbol #16 ⇒ board 16). Beyond the
    /// tabulated rows the pattern continues with all five eliminated levels
    /// behind us: played n ≥ 16 ⇒ effective n + 5 (16 → 21 matches the
    /// table's jump from 18 straight to 21+; every such level resolves to
    /// the A.1 "21+" row anyway).
    ///
    /// Levels are 1-based; 0 is clamped to 1 defensively, and `u32::MAX`
    /// saturates instead of overflowing.
    pub fn effective_level(&self, level: u32, difficulty: Difficulty) -> u32 {
        let level = level.max(1);
        match difficulty {
            Difficulty::Normal => level,
            Difficulty::Hard => match level {
                1 => 2,
                2 => 4,
                3 => 5,
                4..=15 => level + 3,
                _ => level.saturating_add(5),
            },
        }
    }

    /// Resolved spec for a played level (difficulty already applied).
    ///
    /// Effective levels ≥ 21 resolve to the "21+" row (parameters never
    /// change again). Note (Table A.2 prose): the arcade's *displayed* bonus
    /// symbol and its points in hard mode follow the board number, not the
    /// (higher) gameplay level; this method returns the pure gameplay row.
    /// A caller wanting the arcade's lagged fruit display can read the bonus
    /// fields of `level_spec(level, Difficulty::Normal)`, since the symbol
    /// sequence is not eliminated in hard mode.
    pub fn level_spec(&self, level: u32, difficulty: Difficulty) -> &LevelSpec {
        let effective = self.effective_level(level, difficulty);
        // effective_level guarantees >= 1; index 20 is the "21+" row.
        &tables::LEVEL_SPECS[(effective.min(21) - 1) as usize]
    }

    /// Scatter/chase schedule for a played level (difficulty applied).
    ///
    /// Three bands (tables.md, Ch. 2 "Scatter, Chase, Repeat..."): level 1,
    /// levels 2–4, levels 5+. Always eight phases; the final chase has
    /// `ticks: None` (indefinite).
    pub fn schedule(&self, level: u32, difficulty: Difficulty) -> &[SchedulePhase] {
        match self.effective_level(level, difficulty) {
            1 => &tables::SCHEDULE_LEVEL_1,
            2..=4 => &tables::SCHEDULE_LEVELS_2_4,
            _ => &tables::SCHEDULE_LEVELS_5_PLUS,
        }
    }

    /// Per-ghost personal dot-counter limits [Blinky, Pinky, Inky, Clyde]
    /// for a played level (dossier Ch.4; classic values, pre map-scaling).
    ///
    /// Mechanics doc §3.10: Blinky is never subject to the counters (0);
    /// Pinky is always 0; level 1: Inky 30, Clyde 60; level 2: Inky 0,
    /// Clyde 50; level 3+: all 0.
    pub fn house_dot_limits(&self, level: u32, difficulty: Difficulty) -> [u32; 4] {
        match self.effective_level(level, difficulty) {
            1 => [0, 0, 30, 60],
            2 => [0, 0, 0, 50],
            _ => [0, 0, 0, 0],
        }
    }

    /// Global-counter release thresholds (after a death) per ghost.
    ///
    /// Mechanics doc §3.10: Pinky at 7, Inky at 17, Clyde at 32 (with the
    /// deactivation-check quirk owned by the sim). Blinky never waits: 0.
    pub fn global_dot_limits(&self) -> [u32; 4] {
        [0, 7, 17, 32]
    }

    /// Ticks without eating a dot before the next waiting ghost is released.
    ///
    /// Mechanics doc §3.10: 4 seconds on levels 1–4, 3 seconds from level 5
    /// (converted at 60 ticks/second — see module docs).
    pub fn no_dot_release_ticks(&self, level: u32, difficulty: Difficulty) -> u32 {
        if self.effective_level(level, difficulty) <= 4 {
            240
        } else {
            180
        }
    }

    /// Dots eaten at which fruit appears (classic: [70, 170]).
    ///
    /// Mechanics doc §5.2: first fruit at 70 dots cleared, second at 170.
    pub fn fruit_trigger_dots(&self) -> [u32; 2] {
        [70, 170]
    }

    /// Score awarded for eating the nth ghost of one energizer chain (1-based).
    ///
    /// Mechanics doc §5.1: 200 / 400 / 800 / 1600 per energizer chain
    /// (resets to 200 at each new energizer). `chain` is clamped to 1..=4
    /// (classic play has only four ghosts).
    pub fn ghost_chain_score(&self, chain: u8) -> u32 {
        match chain {
            0 | 1 => 200,
            2 => 400,
            3 => 800,
            _ => 1600,
        }
    }

    /// Mechanics doc §5.3: arcade default, one extra life at 10,000 points
    /// (a DIP-switch matter the dossier does not tabulate).
    pub fn extra_life_score(&self) -> u32 {
        10_000
    }
}
