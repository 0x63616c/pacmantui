//! Shared helpers + mini-maps for the `sim_*` integration tests.
//!
//! Included by the other sim test crates via `#[path = "sim_helpers.rs"]`;
//! cargo also compiles it standalone as an (empty) test crate, hence the
//! crate-level `allow(dead_code)`.
//!
//! The mini-maps satisfy every rule in docs/map-format.md "Validation"
//! (door adjacent to a sealed house, pellet reachability, ghost routes,
//! warp pairing) and are designed so the geometry FORCES the behavior under
//! test; expectations in the tests themselves are hand-derived from
//! docs/research/dossier-mechanics.md and docs/research/arcade-supplements.md.
#![allow(dead_code)]

use pacmantui::map::Map;
use pacmantui::rules::Rules;
use pacmantui::sim::Game;
use pacmantui::types::{Difficulty, Dir, Event, GhostId, GhostRender, InputFrame, Sequence};

pub fn classic_game(seed: u64) -> Game {
    Game::new(Map::classic(), Rules::classic(), Difficulty::Normal, seed)
}

pub fn custom_game(seed: u64) -> Game {
    Game::new(Map::custom(), Rules::classic(), Difficulty::Normal, seed)
}

pub fn game_on(src: &str, seed: u64) -> Game {
    let map = Map::parse(src).expect("test mini-map must validate");
    Game::new(map, Rules::classic(), Difficulty::Normal, seed)
}

/// Tick with no input until the sequence is `Playing`; returns the tick
/// count consumed (asserted elsewhere to match the READY! timings).
pub fn run_to_playing(g: &mut Game) -> u32 {
    let mut n = 0;
    while !matches!(g.sequence(), Sequence::Playing) {
        g.tick(InputFrame::default());
        n += 1;
        assert!(n < 100_000, "sequence never reached Playing");
    }
    n
}

/// Run `n` ticks holding `dir`, collecting all events.
pub fn tick_n(g: &mut Game, n: u32, dir: Option<Dir>) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend(g.tick(InputFrame { dir }));
    }
    out
}

pub fn hold(dir: Dir) -> InputFrame {
    InputFrame { dir: Some(dir) }
}

/// Pac-Man's pixel position (whole pixels).
pub fn pac_px(g: &Game) -> (i32, i32) {
    let rs = g.render_state();
    (rs.pac_pos.x.px(), rs.pac_pos.y.px())
}

pub fn ghost(g: &Game, id: GhostId) -> GhostRender {
    let rs = g.render_state();
    *rs.ghosts.iter().find(|gr| gr.id == id).expect("ghost")
}

pub fn ghost_px(g: &Game, id: GhostId) -> (i32, i32) {
    let gr = ghost(g, id);
    (gr.pos.x.px(), gr.pos.y.px())
}

// ---------------------------------------------------------------------------
// Mini-map: "ring" — a single rectangular corridor loop around a sealed
// house. There are NO junctions: every ghost decision is forced, so ghost
// paths are fully deterministic regardless of mode or RNG (frightened picks
// are constrained to the only legal exit). Pellets: 21 dots + 2 energizers
// = 23. `threshold_scale` keeps classic dot thresholds unless overridden.
//
// Geometry (12x7 tiles, 96x56 px):
//   - outer corridor: row 1 (cols 1-10), col 1 (rows 1-5), row 5, col 10
//   - energizers at (1,1) and (10,1)
//   - house interior (3..8,3), door tiles (5,2)+(6,2), center px (48,28)
//   - eyes target (5,1); house exit point px (48,12)
//   - pac spawns at (1,5) px (12,44); Blinky outside at (10,1) px (84,12)
// ---------------------------------------------------------------------------
pub const RING: &str = r#"
format_version = 1
name = "Ring"
id = "ring"
grid = '''
############
#o........o#
#.###--###.#
#.#HHHHHH#.#
#.########.#
#_.........#
############
'''
[spawns]
pac = { tile = [1, 5], facing = "right" }
blinky = { tile = [10, 1], facing = "left" }
pinky = { tile = [5, 3], offset_px = [4, 0], facing = "down" }
inky = { tile = [3, 3], facing = "up" }
clyde = { tile = [7, 3], facing = "up" }
[scatter]
blinky = [11, -1]
pinky = [0, -1]
inky = [11, 7]
clyde = [0, 7]
[house]
center = { tile = [5, 3], offset_px = [4, 0] }
eyes_target = [5, 1]
[fruit]
pos = { tile = [10, 5] }
[rules]
threshold_scale = 1.0
"#;

/// RING with a different `[rules] threshold_scale`, for scaled-threshold
/// scenarios (e.g. 0.001 -> every scaled dot limit rounds to 0).
pub fn ring_with_scale(scale: &str) -> String {
    RING.replace(
        "threshold_scale = 1.0",
        &format!("threshold_scale = {scale}"),
    )
}

// ---------------------------------------------------------------------------
// Mini-map builder: "pass" — the RING with a configurable bottom row and
// Blinky placed ON that row (tile [10,5] + `blinky_offset` px, facing
// Left), for head-on collision scenarios. `scale` is the map's
// threshold_scale: "0.001" rounds every scaled dot threshold to 0
// (docs/map-format.md), which disables Cruise Elroy entirely; "1.0" keeps
// classic values (with few pellets that makes Blinky Elroy-1 = 80% = 1
// px/tick from the start — used deliberately by the death-timing tests).
// ---------------------------------------------------------------------------
pub fn pass_map(blinky_offset: i32, scale: &str, row5: &str) -> String {
    format!(
        r#"
format_version = 1
name = "Pass"
id = "pass"
grid = '''
############
#o........o#
#.###--###.#
#.#HHHHHH#.#
#.########.#
{row5}
############
'''
[spawns]
pac = {{ tile = [1, 5], facing = "right" }}
blinky = {{ tile = [10, 5], offset_px = [{blinky_offset}, 0], facing = "left" }}
pinky = {{ tile = [5, 3], offset_px = [4, 0], facing = "down" }}
inky = {{ tile = [3, 3], facing = "up" }}
clyde = {{ tile = [7, 3], facing = "up" }}
[scatter]
blinky = [11, -1]
pinky = [0, -1]
inky = [11, 7]
clyde = [0, 7]
[house]
center = {{ tile = [5, 3], offset_px = [4, 0] }}
eyes_target = [5, 1]
[fruit]
pos = {{ tile = [9, 1] }}
[rules]
threshold_scale = {scale}
"#
    )
}

// ---------------------------------------------------------------------------
// Mini-map: "sprint" — a two-pellet level for fast level progression.
// Pac (at (28,12), facing Left) eats the (1,1) energizer, turns around,
// crosses the fruit point at (5,1) and clears the level on the (7,1) dot.
// The house opens DOWNWARD onto row 5 (away from pac's runway); with only
// 2 pellets every scaled dot threshold rounds to 0 or 1, so the fruit
// triggers on the first pellet and the ghosts are released immediately.
// ---------------------------------------------------------------------------
pub const SPRINT: &str = r#"
format_version = 1
name = "Sprint"
id = "sprint"
grid = '''
##########
#o_____._#
#_######_#
#_#HHHH#_#
#_##-###_#
#________#
##########
'''
[spawns]
pac = { tile = [3, 1], facing = "left" }
blinky = { tile = [7, 5], facing = "left" }
pinky = { tile = [4, 3], offset_px = [4, 0], facing = "down" }
inky = { tile = [3, 3], facing = "up" }
clyde = { tile = [6, 3], facing = "up" }
[scatter]
blinky = [9, -1]
pinky = [0, -1]
inky = [9, 7]
clyde = [0, 7]
[house]
center = { tile = [4, 3], offset_px = [4, 0] }
eyes_target = [4, 5]
[fruit]
pos = { tile = [5, 1] }
"#;

// ---------------------------------------------------------------------------
// Mini-map: "lab" — the ring extended downward with junctions for ghost
// decision tests. 12 x 9 tiles; no energizers (nothing can go frightened
// and disturb committed decisions).
//
//   Junctions: J = (5,5): exits Left/Right/Down, wall above.
//              M = (1,5): exits Up/Right/Down, wall left.
//              K = (10,5): exits Up/Left/Down, wall right.
//   - Blinky spawns at (5,7) facing Up: his approach to J is forced through
//     (5,6), and his look-ahead decision FOR J is computed on entering
//     (5,6); blinky scatter (5,-1) makes J's Left and Right test tiles tie.
//   - Clyde spawns OUTSIDE at (3,5) facing Left: his decision for M has Up
//     and Down tying (clyde scatter (0,5) sits on M's row).
//   - Pac spawns top row at (3,1) facing Left, on a dot-free corridor,
//     and parks against the wall at (1,1) — away from the action.
//
// `no_up` is spliced into `no_up_tiles = [..]` for red-zone variants.
// ---------------------------------------------------------------------------
pub fn lab_map(no_up: &str) -> String {
    format!(
        r#"
format_version = 1
name = "Lab"
id = "lab"
grid = '''
############
#__________#
#.###--###.#
#.#HHHHHH#.#
#.########.#
#..........#
#.###.####.#
#_.........#
############
'''
no_up_tiles = [{no_up}]
[spawns]
pac = {{ tile = [3, 1], facing = "left" }}
blinky = {{ tile = [5, 7], facing = "up" }}
pinky = {{ tile = [5, 3], offset_px = [4, 0], facing = "down" }}
inky = {{ tile = [3, 3], facing = "up" }}
clyde = {{ tile = [3, 5], facing = "left" }}
[scatter]
blinky = [5, -1]
pinky = [0, -1]
inky = [11, 9]
clyde = [0, 5]
[house]
center = {{ tile = [5, 3], offset_px = [4, 0] }}
eyes_target = [5, 1]
[fruit]
pos = {{ tile = [4, 1] }}
[rules]
threshold_scale = 1.0
"#
    )
}
