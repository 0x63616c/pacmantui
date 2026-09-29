//! Transcription tests for `rules` (W1-RULES).
//!
//! Honesty rule: every expected value in this file is a literal hand-quoted
//! from docs/research/tables.md (Table A.1, Table A.2, the scatter/chase
//! schedule) or docs/research/dossier-mechanics.md (prose-only values:
//! §3.10 house counters/no-dot timer, §5.1 ghost chain, §5.2 fruit triggers,
//! §5.3 extra life). Nothing here is computed from, or imported out of, the
//! constants in src/rules — only the public `Rules` API is called.
//!
//! Seconds → ticks conversion used for the literals: N s = N × 60 ticks
//! (the crate's documented `round(seconds * 60)` rule); "1/60 s" = 1 tick.

use pacmantui::rules::BonusSymbol::{
    Apple, Bell, Cherries, Galaxian, Grapes, Key, Peach, Strawberry,
};
use pacmantui::rules::{BonusSymbol, Rules, SchedulePhase};
use pacmantui::types::{Difficulty, Mode};

type A1Row = (
    u32,
    BonusSymbol,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    u32,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
);

/// Table A.1, hand-quoted from docs/research/tables.md. Columns:
/// (level, bonus symbol, bonus points, Pac-Man speed %, ~Pac-Man dots speed %,
///  ghost speed %, ghost tunnel speed %, Elroy 1 dots left, Elroy 1 speed %,
///  Elroy 2 dots left, Elroy 2 speed %, fright Pac-Man speed %,
///  ~fright Pac-Man dots speed %, fright ghost speed %,
///  fright ticks [seconds × 60], # of flashes).
/// The last row is the source's "21+" row, quoted at level 21 (boundary and
/// beyond-21 behavior are asserted separately).
#[rustfmt::skip]
const TABLE_A1: [A1Row; 21] = [
    ( 1, Cherries,    100,  80, 71, 75, 40,  20,  80, 10,  85, Some( 90), Some(79), Some(50), Some(360) /* 6 s */, Some(5)),
    ( 2, Strawberry,  300,  90, 79, 85, 45,  30,  90, 15,  95, Some( 95), Some(83), Some(55), Some(300) /* 5 s */, Some(5)),
    ( 3, Peach,       500,  90, 79, 85, 45,  40,  90, 20,  95, Some( 95), Some(83), Some(55), Some(240) /* 4 s */, Some(5)),
    ( 4, Peach,       500,  90, 79, 85, 45,  40,  90, 20,  95, Some( 95), Some(83), Some(55), Some(180) /* 3 s */, Some(5)),
    ( 5, Apple,       700, 100, 87, 95, 50,  40, 100, 20, 105, Some(100), Some(87), Some(60), Some(120) /* 2 s */, Some(5)),
    ( 6, Apple,       700, 100, 87, 95, 50,  50, 100, 25, 105, Some(100), Some(87), Some(60), Some(300) /* 5 s */, Some(5)),
    ( 7, Grapes,     1000, 100, 87, 95, 50,  50, 100, 25, 105, Some(100), Some(87), Some(60), Some(120) /* 2 s */, Some(5)),
    ( 8, Grapes,     1000, 100, 87, 95, 50,  50, 100, 25, 105, Some(100), Some(87), Some(60), Some(120) /* 2 s */, Some(5)),
    ( 9, Galaxian,   2000, 100, 87, 95, 50,  60, 100, 30, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (10, Galaxian,   2000, 100, 87, 95, 50,  60, 100, 30, 105, Some(100), Some(87), Some(60), Some(300) /* 5 s */, Some(5)),
    (11, Bell,       3000, 100, 87, 95, 50,  60, 100, 30, 105, Some(100), Some(87), Some(60), Some(120) /* 2 s */, Some(5)),
    (12, Bell,       3000, 100, 87, 95, 50,  80, 100, 40, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (13, Key,        5000, 100, 87, 95, 50,  80, 100, 40, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (14, Key,        5000, 100, 87, 95, 50,  80, 100, 40, 105, Some(100), Some(87), Some(60), Some(180) /* 3 s */, Some(5)),
    (15, Key,        5000, 100, 87, 95, 50, 100, 100, 50, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (16, Key,        5000, 100, 87, 95, 50, 100, 100, 50, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (17, Key,        5000, 100, 87, 95, 50, 100, 100, 50, 105, None,      None,     None,     None,               None),
    (18, Key,        5000, 100, 87, 95, 50, 100, 100, 50, 105, Some(100), Some(87), Some(60), Some( 60) /* 1 s */, Some(3)),
    (19, Key,        5000, 100, 87, 95, 50, 120, 100, 60, 105, None,      None,     None,     None,               None),
    (20, Key,        5000, 100, 87, 95, 50, 120, 100, 60, 105, None,      None,     None,     None,               None),
    (21, Key,        5000,  90, 79, 95, 50, 120, 100, 60, 105, None,      None,     None,     None,               None),
];

fn assert_spec_matches(rules: &Rules, level: u32, difficulty: Difficulty, row: &A1Row) {
    let (
        row_level,
        symbol,
        points,
        pac,
        pac_dots,
        ghost,
        tunnel,
        e1_dots,
        e1,
        e2_dots,
        e2,
        f_pac,
        f_pac_dots,
        f_ghost,
        f_ticks,
        f_flashes,
    ) = *row;
    let s = rules.level_spec(level, difficulty);
    let ctx = format!("played level {level} ({difficulty:?}) vs A.1 row {row_level}");
    assert_eq!(s.bonus_symbol, symbol, "bonus symbol, {ctx}");
    assert_eq!(s.bonus_points, points, "bonus points, {ctx}");
    assert_eq!(s.pac_speed_pct, pac, "Pac-Man speed, {ctx}");
    assert_eq!(s.pac_dots_speed_pct, pac_dots, "Pac-Man dots speed, {ctx}");
    assert_eq!(s.ghost_speed_pct, ghost, "ghost speed, {ctx}");
    assert_eq!(
        s.ghost_tunnel_speed_pct, tunnel,
        "ghost tunnel speed, {ctx}"
    );
    assert_eq!(s.elroy1_dots_left, e1_dots, "Elroy 1 dots left, {ctx}");
    assert_eq!(s.elroy1_speed_pct, e1, "Elroy 1 speed, {ctx}");
    assert_eq!(s.elroy2_dots_left, e2_dots, "Elroy 2 dots left, {ctx}");
    assert_eq!(s.elroy2_speed_pct, e2, "Elroy 2 speed, {ctx}");
    assert_eq!(s.fright_pac_speed_pct, f_pac, "fright Pac-Man speed, {ctx}");
    assert_eq!(
        s.fright_pac_dots_speed_pct, f_pac_dots,
        "fright Pac-Man dots speed, {ctx}"
    );
    assert_eq!(
        s.fright_ghost_speed_pct, f_ghost,
        "fright ghost speed, {ctx}"
    );
    assert_eq!(s.fright_ticks, f_ticks, "fright ticks, {ctx}");
    assert_eq!(s.fright_flashes, f_flashes, "fright flashes, {ctx}");
}

#[test]
fn table_a1_every_row_normal() {
    let rules = Rules::classic();
    for row in &TABLE_A1 {
        assert_spec_matches(&rules, row.0, Difficulty::Normal, row);
    }
}

#[test]
fn table_a1_21_plus_row_applies_to_every_later_level() {
    let rules = Rules::classic();
    let row_21_plus = &TABLE_A1[20];
    for level in [21, 22, 30, 100, 255, 1_000_000, u32::MAX] {
        assert_spec_matches(&rules, level, Difficulty::Normal, row_21_plus);
    }
}

#[test]
fn table_a1_boundaries() {
    let rules = Rules::classic();
    let n = Difficulty::Normal;
    // 4 → 5: Pac-Man 90 → 100, ghost 85 → 95, tunnel 45 → 50 (tables.md).
    assert_eq!(rules.level_spec(4, n).pac_speed_pct, 90);
    assert_eq!(rules.level_spec(5, n).pac_speed_pct, 100);
    assert_eq!(rules.level_spec(4, n).ghost_speed_pct, 85);
    assert_eq!(rules.level_spec(5, n).ghost_speed_pct, 95);
    assert_eq!(rules.level_spec(4, n).ghost_tunnel_speed_pct, 45);
    assert_eq!(rules.level_spec(5, n).ghost_tunnel_speed_pct, 50);
    // 20 → 21: Pac-Man drops back 100 → 90 (~87 → ~79 dots) while ghosts stay 95.
    assert_eq!(rules.level_spec(20, n).pac_speed_pct, 100);
    assert_eq!(rules.level_spec(21, n).pac_speed_pct, 90);
    assert_eq!(rules.level_spec(20, n).pac_dots_speed_pct, 87);
    assert_eq!(rules.level_spec(21, n).pac_dots_speed_pct, 79);
    assert_eq!(rules.level_spec(20, n).ghost_speed_pct, 95);
    assert_eq!(rules.level_spec(21, n).ghost_speed_pct, 95);
    // Level 100 is the 21+ row.
    assert_eq!(rules.level_spec(100, n).pac_speed_pct, 90);
    assert_eq!(rules.level_spec(100, n).bonus_points, 5000);
}

#[test]
fn no_frightened_mode_levels() {
    // tables.md: levels 17, 19, 20, and 21+ have all five fright columns
    // dashed — ghosts never turn blue (they still reverse direction).
    let rules = Rules::classic();
    for level in [17, 19, 20, 21, 22, 100, u32::MAX] {
        let s = rules.level_spec(level, Difficulty::Normal);
        assert_eq!(s.fright_pac_speed_pct, None, "level {level}");
        assert_eq!(s.fright_pac_dots_speed_pct, None, "level {level}");
        assert_eq!(s.fright_ghost_speed_pct, None, "level {level}");
        assert_eq!(s.fright_ticks, None, "level {level}");
        assert_eq!(s.fright_flashes, None, "level {level}");
    }
    // Level 18 in between still has a 1 s / 3-flash frightened period.
    let s18 = rules.level_spec(18, Difficulty::Normal);
    assert_eq!(s18.fright_ticks, Some(60));
    assert_eq!(s18.fright_flashes, Some(3));
}

// --- Scatter/chase schedule (tables.md, Ch. 2 "Scatter, Chase, Repeat...") ---

/// Column "Level 1": 7 / 20 / 7 / 20 / 5 / 20 / 5 / indefinite (seconds).
const SCHEDULE_LEVEL_1: [(Mode, Option<u32>); 8] = [
    (Mode::Scatter, Some(420)), // 7 s
    (Mode::Chase, Some(1200)),  // 20 s
    (Mode::Scatter, Some(420)), // 7 s
    (Mode::Chase, Some(1200)),  // 20 s
    (Mode::Scatter, Some(300)), // 5 s
    (Mode::Chase, Some(1200)),  // 20 s
    (Mode::Scatter, Some(300)), // 5 s
    (Mode::Chase, None),        // indefinite
];

/// Column "Levels 2–4": 7 / 20 / 7 / 20 / 5 / 1033 / 1/60 / indefinite.
const SCHEDULE_LEVELS_2_4: [(Mode, Option<u32>); 8] = [
    (Mode::Scatter, Some(420)),  // 7 s
    (Mode::Chase, Some(1200)),   // 20 s
    (Mode::Scatter, Some(420)),  // 7 s
    (Mode::Chase, Some(1200)),   // 20 s
    (Mode::Scatter, Some(300)),  // 5 s
    (Mode::Chase, Some(61_980)), // 1033 s
    (Mode::Scatter, Some(1)),    // 1/60 s = one tick
    (Mode::Chase, None),         // indefinite
];

/// Column "Levels 5+": 5 / 20 / 5 / 20 / 5 / 1037 / 1/60 / indefinite.
const SCHEDULE_LEVELS_5_PLUS: [(Mode, Option<u32>); 8] = [
    (Mode::Scatter, Some(300)),  // 5 s
    (Mode::Chase, Some(1200)),   // 20 s
    (Mode::Scatter, Some(300)),  // 5 s
    (Mode::Chase, Some(1200)),   // 20 s
    (Mode::Scatter, Some(300)),  // 5 s
    (Mode::Chase, Some(62_220)), // 1037 s
    (Mode::Scatter, Some(1)),    // 1/60 s = one tick
    (Mode::Chase, None),         // indefinite
];

fn assert_schedule(actual: &[SchedulePhase], expected: &[(Mode, Option<u32>)], ctx: &str) {
    assert_eq!(actual.len(), expected.len(), "phase count, {ctx}");
    for (i, (phase, (mode, ticks))) in actual.iter().zip(expected).enumerate() {
        assert_eq!(phase.mode, *mode, "phase {} mode, {ctx}", i + 1);
        assert_eq!(phase.ticks, *ticks, "phase {} ticks, {ctx}", i + 1);
    }
}

#[test]
fn schedule_all_three_bands() {
    let rules = Rules::classic();
    let n = Difficulty::Normal;
    assert_schedule(rules.schedule(1, n), &SCHEDULE_LEVEL_1, "level 1");
    for level in [2, 3, 4] {
        assert_schedule(rules.schedule(level, n), &SCHEDULE_LEVELS_2_4, "levels 2-4");
    }
    for level in [5, 6, 20, 21, 100, u32::MAX] {
        assert_schedule(
            rules.schedule(level, n),
            &SCHEDULE_LEVELS_5_PLUS,
            "levels 5+",
        );
    }
}

#[test]
fn schedule_hard_uses_effective_level() {
    let rules = Rules::classic();
    // Hard board 1 is level-2 gameplay (Table A.2) → the 2-4 band; board 2 is
    // level 4 → still 2-4; board 3 is level 5 → the 5+ band. Level 1's band is
    // unreachable in hard difficulty.
    assert_schedule(
        rules.schedule(1, Difficulty::Hard),
        &SCHEDULE_LEVELS_2_4,
        "hard board 1",
    );
    assert_schedule(
        rules.schedule(2, Difficulty::Hard),
        &SCHEDULE_LEVELS_2_4,
        "hard board 2",
    );
    assert_schedule(
        rules.schedule(3, Difficulty::Hard),
        &SCHEDULE_LEVELS_5_PLUS,
        "hard board 3",
    );
}

// --- Table A.2 difficulty mapping (tables.md) ---

#[test]
fn effective_level_normal_is_identity() {
    let rules = Rules::classic();
    for level in 1..=30 {
        assert_eq!(rules.effective_level(level, Difficulty::Normal), level);
    }
    assert_eq!(
        rules.effective_level(u32::MAX, Difficulty::Normal),
        u32::MAX
    );
}

#[test]
fn effective_level_hard_every_table_a2_row() {
    let rules = Rules::classic();
    // Table A.2's Hard column, top to bottom, skipping the "–" rows (levels
    // 1, 3, 6, 19, 20 are eliminated from play): 2, 4, 5, 7, 8, 9, 10, 11,
    // 12, 13, 14, 15, 16, 17, 18, then 21+. Hard's Nth board is the Nth
    // entry of that sequence.
    let hard_sequence = [2, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 21];
    for (i, expected) in hard_sequence.into_iter().enumerate() {
        let played = i as u32 + 1;
        assert_eq!(
            rules.effective_level(played, Difficulty::Hard),
            expected,
            "hard board {played}"
        );
    }
    // The eliminated levels never occur as hard gameplay levels.
    for played in 1..=60 {
        let eff = rules.effective_level(played, Difficulty::Hard);
        assert!(
            ![1, 3, 6, 19, 20].contains(&eff),
            "hard board {played} resolved to eliminated level {eff}"
        );
    }
}

#[test]
fn effective_level_hard_beyond_table_continuation() {
    let rules = Rules::classic();
    // Past the tabulated rows all five eliminated levels are behind us, so
    // hard board n maps to normal n + 5 (16 → 21 matches the table's jump
    // from 18 straight to 21+; every such level uses the A.1 "21+" row).
    assert_eq!(rules.effective_level(16, Difficulty::Hard), 21);
    assert_eq!(rules.effective_level(17, Difficulty::Hard), 22);
    assert_eq!(rules.effective_level(18, Difficulty::Hard), 23);
    assert_eq!(rules.effective_level(20, Difficulty::Hard), 25);
    assert_eq!(rules.effective_level(100, Difficulty::Hard), 105);
    // u32::MAX saturates instead of overflowing.
    assert_eq!(rules.effective_level(u32::MAX, Difficulty::Hard), u32::MAX);
}

#[test]
fn level_spec_hard_returns_effective_row() {
    let rules = Rules::classic();
    // Hard board 1 is level-2 gameplay: Strawberry 300, Pac-Man 90%,
    // ghost 85%, fright 5 s (300 ticks). (The arcade's *displayed* symbol
    // lags — that presentation quirk is out of scope for level_spec.)
    let s = rules.level_spec(1, Difficulty::Hard);
    assert_eq!(s.bonus_symbol, Strawberry);
    assert_eq!(s.bonus_points, 300);
    assert_eq!(s.pac_speed_pct, 90);
    assert_eq!(s.ghost_speed_pct, 85);
    assert_eq!(s.fright_ticks, Some(300));
    // Hard board 16 is 21+ gameplay: Pac-Man back to 90%, no frightened mode.
    let s = rules.level_spec(16, Difficulty::Hard);
    assert_eq!(s.pac_speed_pct, 90);
    assert_eq!(s.ghost_speed_pct, 95);
    assert_eq!(s.fright_ticks, None);
    assert_eq!(
        rules.level_spec(u32::MAX, Difficulty::Hard),
        rules.level_spec(21, Difficulty::Normal)
    );
}

// --- Ghost house counters and no-dot timer (mechanics doc §3.10) ---

#[test]
fn house_dot_limits_classic() {
    let rules = Rules::classic();
    let n = Difficulty::Normal;
    // [Blinky, Pinky, Inky, Clyde]; Blinky is never subject (0), Pinky is
    // always 0. Level 1: Inky 30, Clyde 60. Level 2: Inky 0, Clyde 50.
    // Level 3+: all 0.
    assert_eq!(rules.house_dot_limits(1, n), [0, 0, 30, 60]);
    assert_eq!(rules.house_dot_limits(2, n), [0, 0, 0, 50]);
    for level in [3, 4, 5, 21, 100, u32::MAX] {
        assert_eq!(
            rules.house_dot_limits(level, n),
            [0, 0, 0, 0],
            "level {level}"
        );
    }
    // Hard board 1 is level-2 gameplay; board 2 is level 4. Level 1's
    // 30/60 limits are unreachable in hard difficulty.
    assert_eq!(rules.house_dot_limits(1, Difficulty::Hard), [0, 0, 0, 50]);
    assert_eq!(rules.house_dot_limits(2, Difficulty::Hard), [0, 0, 0, 0]);
}

#[test]
fn global_dot_limits_after_death() {
    // Pinky at 7, Inky at 17, Clyde at 32; Blinky never waits (0).
    assert_eq!(Rules::classic().global_dot_limits(), [0, 7, 17, 32]);
}

#[test]
fn no_dot_release_timer() {
    let rules = Rules::classic();
    let n = Difficulty::Normal;
    // 4 seconds on levels 1-4 (240 ticks), 3 seconds from level 5 (180).
    for level in [1, 2, 3, 4] {
        assert_eq!(rules.no_dot_release_ticks(level, n), 240, "level {level}");
    }
    for level in [5, 6, 20, 21, 100, u32::MAX] {
        assert_eq!(rules.no_dot_release_ticks(level, n), 180, "level {level}");
    }
    // Hard boards 1 and 2 are gameplay levels 2 and 4 (still 4 s); board 3
    // is level 5 (3 s).
    assert_eq!(rules.no_dot_release_ticks(1, Difficulty::Hard), 240);
    assert_eq!(rules.no_dot_release_ticks(2, Difficulty::Hard), 240);
    assert_eq!(rules.no_dot_release_ticks(3, Difficulty::Hard), 180);
}

// --- Fruit, scoring (mechanics doc §5.1-5.3) ---

#[test]
fn fruit_triggers() {
    // First fruit at 70 dots cleared, second at 170.
    assert_eq!(Rules::classic().fruit_trigger_dots(), [70, 170]);
}

#[test]
fn ghost_chain_scores() {
    let rules = Rules::classic();
    // 200 / 400 / 800 / 1600 for successive ghosts from one energizer.
    assert_eq!(rules.ghost_chain_score(1), 200);
    assert_eq!(rules.ghost_chain_score(2), 400);
    assert_eq!(rules.ghost_chain_score(3), 800);
    assert_eq!(rules.ghost_chain_score(4), 1600);
}

#[test]
fn extra_life_at_10_000() {
    assert_eq!(Rules::classic().extra_life_score(), 10_000);
}
