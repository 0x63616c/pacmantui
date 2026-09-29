//! Static table data transcribed from docs/research/tables.md / tables.json
//! (The Pac-Man Dossier, Appendix A and Chapter 2 body tables).
//!
//! Private to `rules`: all access goes through the `Rules` API. Integration
//! tests quote their expected values from the docs directly and must never
//! import these statics (honesty rule, see module docs in `rules`).

use super::{BonusSymbol, LevelSpec, SchedulePhase};
use crate::types::Mode;

/// Seconds → ticks: `round(seconds * 60)`.
///
/// Every duration cell in the source tables is an integral number of seconds
/// (the sole sub-second value, the "1/60 s" scatter phase, is encoded directly
/// as 1 tick below), so this is exactly `seconds * 60`.
///
/// Interpretation (tables.md, mechanics doc §8.3): the arcade counts frames
/// and its real frame rate is 60.606061 Hz — tables.md quotes Table A.1's
/// title "100% speed = 75.75757625 pixels/sec", i.e. 1.25 px/frame at
/// 60.606061 Hz — so N "seconds" of table time elapse in ~1% less wall-clock
/// time than N seconds. The dossier's own prose already conflates "1/60th of
/// a second" with one frame; we follow that frame-counting reading (60
/// ticks per table-second) rather than rescaling to wall-clock.
const fn secs_to_ticks(seconds: u32) -> u32 {
    seconds * 60
}

/// Builds one Table A.1 row; arguments follow the source's column order.
///
/// `fright` is `Some((pac_pct, pac_dots_pct, ghost_pct, seconds, flashes))`
/// or `None` where the source dashes ("–") all five fright columns together
/// (levels 17, 19, 20, 21+: ghosts never turn blue, energizers still reverse
/// them). Encoding the five columns as one `Option` preserves the source's
/// all-or-nothing invariant. `seconds` is converted via [`secs_to_ticks`].
#[allow(clippy::too_many_arguments)] // positional args deliberately mirror the source's column order
const fn row(
    bonus_symbol: BonusSymbol,
    bonus_points: u32,
    pac_speed_pct: u32,
    pac_dots_speed_pct: u32,
    ghost_speed_pct: u32,
    ghost_tunnel_speed_pct: u32,
    elroy1_dots_left: u32,
    elroy1_speed_pct: u32,
    elroy2_dots_left: u32,
    elroy2_speed_pct: u32,
    fright: Option<(u32, u32, u32, u32, u32)>,
) -> LevelSpec {
    let (
        fright_pac_speed_pct,
        fright_pac_dots_speed_pct,
        fright_ghost_speed_pct,
        fright_ticks,
        fright_flashes,
    ) = match fright {
        Some((pac, pac_dots, ghost, seconds, flashes)) => (
            Some(pac),
            Some(pac_dots),
            Some(ghost),
            Some(secs_to_ticks(seconds)),
            Some(flashes),
        ),
        None => (None, None, None, None, None),
    };
    LevelSpec {
        bonus_symbol,
        bonus_points,
        pac_speed_pct,
        pac_dots_speed_pct,
        ghost_speed_pct,
        ghost_tunnel_speed_pct,
        elroy1_dots_left,
        elroy1_speed_pct,
        elroy2_dots_left,
        elroy2_speed_pct,
        fright_pac_speed_pct,
        fright_pac_dots_speed_pct,
        fright_ghost_speed_pct,
        fright_ticks,
        fright_flashes,
    }
}

use BonusSymbol::{Apple, Bell, Cherries, Galaxian, Grapes, Key, Peach, Strawberry};

/// Table A.1 — Level Specifications (docs/research/tables.md).
///
/// Index 0 = level 1 … index 19 = level 20; index 20 = the "21+" row, which
/// applies to level 21 and every level after (parameters never change again —
/// note the deliberate exception: Pac-Man drops back to 90% speed at 21+
/// while ghosts stay at 95%).
///
/// Row layout mirrors the source columns:
/// `row(symbol, points, pac%, ~pac-dots%, ghost%, tunnel%,
///      elroy1-dots, elroy1%, elroy2-dots, elroy2%,
///      fright: (pac%, ~pac-dots%, ghost%, seconds, flashes))`.
#[rustfmt::skip]
pub(super) static LEVEL_SPECS: [LevelSpec; 21] = [
    /*  1 */ row(Cherries,   100,  80, 71, 75, 40,  20,  80, 10,  85, Some((90, 79, 50, 6, 5))),
    /*  2 */ row(Strawberry, 300,  90, 79, 85, 45,  30,  90, 15,  95, Some((95, 83, 55, 5, 5))),
    /*  3 */ row(Peach,      500,  90, 79, 85, 45,  40,  90, 20,  95, Some((95, 83, 55, 4, 5))),
    /*  4 */ row(Peach,      500,  90, 79, 85, 45,  40,  90, 20,  95, Some((95, 83, 55, 3, 5))),
    /*  5 */ row(Apple,      700, 100, 87, 95, 50,  40, 100, 20, 105, Some((100, 87, 60, 2, 5))),
    /*  6 */ row(Apple,      700, 100, 87, 95, 50,  50, 100, 25, 105, Some((100, 87, 60, 5, 5))),
    /*  7 */ row(Grapes,    1000, 100, 87, 95, 50,  50, 100, 25, 105, Some((100, 87, 60, 2, 5))),
    /*  8 */ row(Grapes,    1000, 100, 87, 95, 50,  50, 100, 25, 105, Some((100, 87, 60, 2, 5))),
    /*  9 */ row(Galaxian,  2000, 100, 87, 95, 50,  60, 100, 30, 105, Some((100, 87, 60, 1, 3))),
    /* 10 */ row(Galaxian,  2000, 100, 87, 95, 50,  60, 100, 30, 105, Some((100, 87, 60, 5, 5))),
    /* 11 */ row(Bell,      3000, 100, 87, 95, 50,  60, 100, 30, 105, Some((100, 87, 60, 2, 5))),
    /* 12 */ row(Bell,      3000, 100, 87, 95, 50,  80, 100, 40, 105, Some((100, 87, 60, 1, 3))),
    /* 13 */ row(Key,       5000, 100, 87, 95, 50,  80, 100, 40, 105, Some((100, 87, 60, 1, 3))),
    /* 14 */ row(Key,       5000, 100, 87, 95, 50,  80, 100, 40, 105, Some((100, 87, 60, 3, 5))),
    /* 15 */ row(Key,       5000, 100, 87, 95, 50, 100, 100, 50, 105, Some((100, 87, 60, 1, 3))),
    /* 16 */ row(Key,       5000, 100, 87, 95, 50, 100, 100, 50, 105, Some((100, 87, 60, 1, 3))),
    /* 17 */ row(Key,       5000, 100, 87, 95, 50, 100, 100, 50, 105, None),
    /* 18 */ row(Key,       5000, 100, 87, 95, 50, 100, 100, 50, 105, Some((100, 87, 60, 1, 3))),
    /* 19 */ row(Key,       5000, 100, 87, 95, 50, 120, 100, 60, 105, None),
    /* 20 */ row(Key,       5000, 100, 87, 95, 50, 120, 100, 60, 105, None),
    /* 21+ */row(Key,       5000,  90, 79, 95, 50, 120, 100, 60, 105, None),
];

const fn scatter_secs(seconds: u32) -> SchedulePhase {
    SchedulePhase {
        mode: Mode::Scatter,
        ticks: Some(secs_to_ticks(seconds)),
    }
}

const fn chase_secs(seconds: u32) -> SchedulePhase {
    SchedulePhase {
        mode: Mode::Chase,
        ticks: Some(secs_to_ticks(seconds)),
    }
}

/// The source's "1/60" scatter phase: exactly one frame = one tick (it
/// appears in play as a simple direction reversal).
const SCATTER_ONE_TICK: SchedulePhase = SchedulePhase {
    mode: Mode::Scatter,
    ticks: Some(1),
};

/// The source's "indefinite" final phase: chase for the rest of the level.
const CHASE_INDEFINITE: SchedulePhase = SchedulePhase {
    mode: Mode::Chase,
    ticks: None,
};

/// Scatter/chase schedule, level 1 band (tables.md, Ch. 2 body table;
/// column "Level 1": 7 / 20 / 7 / 20 / 5 / 20 / 5 / indefinite, in seconds).
pub(super) static SCHEDULE_LEVEL_1: [SchedulePhase; 8] = [
    scatter_secs(7),
    chase_secs(20),
    scatter_secs(7),
    chase_secs(20),
    scatter_secs(5),
    chase_secs(20),
    scatter_secs(5),
    CHASE_INDEFINITE,
];

/// Scatter/chase schedule, levels 2–4 band (column "Levels 2–4":
/// 7 / 20 / 7 / 20 / 5 / 1033 / 1/60 / indefinite, in seconds).
pub(super) static SCHEDULE_LEVELS_2_4: [SchedulePhase; 8] = [
    scatter_secs(7),
    chase_secs(20),
    scatter_secs(7),
    chase_secs(20),
    scatter_secs(5),
    chase_secs(1033),
    SCATTER_ONE_TICK,
    CHASE_INDEFINITE,
];

/// Scatter/chase schedule, levels 5+ band (column "Levels 5+":
/// 5 / 20 / 5 / 20 / 5 / 1037 / 1/60 / indefinite, in seconds).
pub(super) static SCHEDULE_LEVELS_5_PLUS: [SchedulePhase; 8] = [
    scatter_secs(5),
    chase_secs(20),
    scatter_secs(5),
    chase_secs(20),
    scatter_secs(5),
    chase_secs(1037),
    SCATTER_ONE_TICK,
    CHASE_INDEFINITE,
];
