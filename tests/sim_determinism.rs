//! Determinism and safety properties: identical seed + inputs must
//! reproduce the exact event log and final state (sim module docs: fixed
//! ghost order, integer math, two documented RNG consumption points), and
//! long random runs must never break the board invariants.

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::map::{Cell, Map};
use pacmantui::rules::Rules;
use pacmantui::sim::Game;
use pacmantui::types::{Difficulty, Dir, Event, InputFrame};

/// Test-local xorshift for INPUT generation (never the sim's RNG).
struct TestRng(u32);

impl TestRng {
    fn next(&mut self) -> u32 {
        let mut x = self.0.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}

fn scripted_dir(t: u32) -> Option<Dir> {
    match (t / 37) % 5 {
        0 => Some(Dir::Left),
        1 => Some(Dir::Up),
        2 => Some(Dir::Right),
        3 => Some(Dir::Down),
        _ => None,
    }
}

/// Same seed + same inputs ⇒ identical event log and identical full game
/// state (compared through the Debug rendering, which covers every field
/// including the RNG).
#[test]
fn same_seed_same_inputs_identical_run() {
    let run = |map: Map| {
        let mut g = Game::new(map, Rules::classic(), Difficulty::Normal, 0xC0FFEE);
        let mut log: Vec<(u32, Event)> = Vec::new();
        for t in 0..6000u32 {
            for e in g.tick(InputFrame {
                dir: scripted_dir(t),
            }) {
                log.push((t, e));
            }
        }
        (log, format!("{g:?}"))
    };
    let (log_a, state_a) = run(Map::classic());
    let (log_b, state_b) = run(Map::classic());
    assert!(
        log_a
            .iter()
            .any(|(_, e)| matches!(e, Event::DotEaten { .. })),
        "the scripted run must actually play"
    );
    assert_eq!(log_a, log_b, "event logs diverged");
    assert_eq!(state_a, state_b, "final states diverged");
    // And on the custom map.
    let run_c = |seed: u64| {
        let mut g = Game::new(Map::custom(), Rules::classic(), Difficulty::Normal, seed);
        let mut log = Vec::new();
        for t in 0..6000u32 {
            log.extend(g.tick(InputFrame {
                dir: scripted_dir(t),
            }));
        }
        (log, format!("{g:?}"))
    };
    let a = run_c(7);
    let b = run_c(7);
    assert_eq!(a, b, "custom-map run diverged");
}

/// 20,000 random-input ticks: no actor ever occupies an illegal tile, the
/// score is monotonically non-decreasing, and no pellet scores twice
/// within a level. A finished game is restarted with a fresh seed so all
/// 20k ticks exercise live play.
fn property_run(mut map_fn: impl FnMut() -> Map, seed: u32) {
    let mut rng = TestRng(seed);
    let mut g = Game::new(
        map_fn(),
        Rules::classic(),
        Difficulty::Normal,
        u64::from(seed),
    );
    let dims = (g.map().width() * g.map().height()) as usize;
    let mut eaten_this_level = vec![false; dims];
    let mut held: Option<Dir> = None;
    let mut last_score = 0u32;
    for _ in 0..20_000u32 {
        if g.is_game_over() {
            let s = u64::from(rng.next());
            g = Game::new(map_fn(), Rules::classic(), Difficulty::Normal, s);
            eaten_this_level.fill(false);
            last_score = 0;
        }
        if rng.next().is_multiple_of(13) {
            held = match rng.next() % 6 {
                0 => Some(Dir::Up),
                1 => Some(Dir::Left),
                2 => Some(Dir::Down),
                3 => Some(Dir::Right),
                _ => held,
            };
        }
        let ev = g.tick(InputFrame { dir: held });
        for e in &ev {
            match e {
                Event::DotEaten { tile, .. } | Event::EnergizerEaten { tile, .. } => {
                    let i = g.map().tile_index(*tile);
                    assert!(!eaten_this_level[i], "pellet at {tile:?} scored twice");
                    eaten_this_level[i] = true;
                }
                Event::NextLevelStarted { .. } => eaten_this_level.fill(false),
                _ => {}
            }
        }
        let s = g.score();
        assert!(s >= last_score, "score went backwards: {last_score} -> {s}");
        last_score = s;
        let rs = g.render_state();
        assert!(
            g.map().walkable(rs.pac_pos.tile()),
            "pac on illegal tile {:?}",
            rs.pac_pos.tile()
        );
        for gh in &rs.ghosts {
            assert_ne!(
                g.map().cell(gh.pos.tile()),
                Cell::Wall,
                "{:?} inside a wall at {:?} ({:?})",
                gh.id,
                gh.pos.tile(),
                gh.state
            );
        }
    }
}

#[test]
fn property_20k_random_ticks_classic() {
    property_run(Map::classic, 0x1234_5678);
}

#[test]
fn property_20k_random_ticks_custom() {
    property_run(Map::custom, 0x0BAD_F00D);
}
