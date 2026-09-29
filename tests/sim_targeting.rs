//! Ghost targeting (dossier §3.5/§3.6), decision tie-breaks (§3.4), and
//! red zones (§3.7) with the frightened/eyes exemptions (supplements §12).
//!
//! `chase_target` expectations are computed by hand from the dossier's
//! formulas; the junction tests use the LAB mini-map whose geometry forces
//! the approach, so the observed turn IS the decision under test.

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::sim::chase_target;
use pacmantui::types::{Dir, Event, GhostId, GhostState, Mode, TilePos};

const T: fn(i32, i32) -> TilePos = TilePos::new;

/// Blinky targets Pac-Man's tile directly (dossier §3.6 "Blinky").
#[test]
fn blinky_targets_pac_tile() {
    for d in Dir::IN_PRIORITY_ORDER {
        assert_eq!(
            chase_target(GhostId::Blinky, T(14, 26), d, T(10, 10), T(0, 0), T(25, -1)),
            T(14, 26)
        );
    }
}

/// Pinky: 4 tiles ahead — except facing Up, where the overflow bug makes it
/// 4 up AND 4 left (dossier §3.6 "Pinky", quoted verbatim there).
#[test]
fn pinky_four_ahead_with_up_overflow() {
    let pac = T(14, 26);
    let b = T(0, 0);
    let sc = T(2, -1);
    let own = T(5, 5);
    assert_eq!(
        chase_target(GhostId::Pinky, pac, Dir::Left, b, own, sc),
        T(10, 26)
    );
    assert_eq!(
        chase_target(GhostId::Pinky, pac, Dir::Right, b, own, sc),
        T(18, 26)
    );
    assert_eq!(
        chase_target(GhostId::Pinky, pac, Dir::Down, b, own, sc),
        T(14, 30)
    );
    // Overflow: 4 up AND 4 left.
    assert_eq!(
        chase_target(GhostId::Pinky, pac, Dir::Up, b, own, sc),
        T(10, 22)
    );
}

/// Inky: pivot 2 ahead of Pac-Man (same Up overflow: 2 up AND 2 left), then
/// double the vector from Blinky to the pivot: target = 2*pivot − blinky
/// (dossier §3.6 "Inky").
#[test]
fn inky_pivot_and_doubled_vector_with_up_overflow() {
    // Pac at (14,26) facing Left: pivot (12,26); Blinky at (21,26):
    // target = (2*12-21, 2*26-26) = (3, 26).
    assert_eq!(
        chase_target(
            GhostId::Inky,
            T(14, 26),
            Dir::Left,
            T(21, 26),
            T(0, 0),
            T(27, 34)
        ),
        T(3, 26)
    );
    // Facing Up: pivot = (14-2, 26-2) = (12,24) via the overflow; Blinky at
    // (14,14): target = (2*12-14, 2*24-14) = (10, 34).
    assert_eq!(
        chase_target(
            GhostId::Inky,
            T(14, 26),
            Dir::Up,
            T(14, 14),
            T(0, 0),
            T(27, 34)
        ),
        T(10, 34)
    );
    // Facing Down, no overflow: pivot (14,28), Blinky (10,20):
    // target = (18, 36).
    assert_eq!(
        chase_target(
            GhostId::Inky,
            T(14, 26),
            Dir::Down,
            T(10, 20),
            T(0, 0),
            T(27, 34)
        ),
        T(18, 36)
    );
}

/// Clyde: Pac-Man's tile at Euclidean distance >= 8 tiles, else his scatter
/// corner (dossier §3.6 "Clyde"). Distance exactly 8 counts as far.
#[test]
fn clyde_eight_tile_switch() {
    let sc = T(0, 34);
    // dist = 8 exactly (vertical): far -> pac tile.
    assert_eq!(
        chase_target(GhostId::Clyde, T(14, 26), Dir::Left, T(0, 0), T(14, 18), sc),
        T(14, 26)
    );
    // dist = 7: near -> scatter corner.
    assert_eq!(
        chase_target(GhostId::Clyde, T(14, 26), Dir::Left, T(0, 0), T(14, 19), sc),
        sc
    );
    // Euclidean metric: (6,5) offset -> sqrt(61) < 8 -> near.
    assert_eq!(
        chase_target(GhostId::Clyde, T(14, 26), Dir::Left, T(0, 0), T(20, 21), sc),
        sc
    );
}

/// Live game targeting: scatter targets the corner tile, chase targets per
/// ghost — observed through the `ghost_target` getter on the classic map
/// (dossier §3.2/§3.5; scatter corners ROM-confirmed in supplements §8).
#[test]
fn scatter_then_chase_targets_on_classic() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    // Scatter (first 420 ticks): corners from maps/classic.pmtoml. Housed
    // ghosts have no pathfinding target (getter contract).
    assert_eq!(g.mode(), Mode::Scatter);
    assert_eq!(g.ghost_target(GhostId::Blinky), Some(T(25, -1)));
    assert_eq!(g.ghost_target(GhostId::Inky), None, "housed: no target");
    assert_eq!(g.ghost_target(GhostId::Clyde), None, "housed: no target");
    // Pinky (dot limit 0) exits promptly: house climb is 24 px at 0.5
    // px/tick = 48 ticks (supplements §10); Active and corner-bound after.
    let _ = h::tick_n(&mut g, 60, Some(Dir::Left));
    assert_eq!(g.ghost_target(GhostId::Pinky), Some(T(2, -1)));
    // Enter chase holding Left; pac's tile/facing then feed the formulas.
    let _ = h::tick_n(&mut g, 361, Some(Dir::Left));
    assert_eq!(g.mode(), Mode::Chase);
    let rs = g.render_state();
    let pac = rs.pac_pos.tile();
    let blinky = rs.ghosts[0].pos.tile();
    assert_eq!(g.ghost_target(GhostId::Blinky), Some(pac));
    assert_eq!(
        g.ghost_target(GhostId::Pinky),
        Some(chase_target(
            GhostId::Pinky,
            pac,
            rs.pac_dir,
            blinky,
            rs.ghosts[1].pos.tile(),
            T(2, -1)
        ))
    );
}

/// Tie-break Left over Right (dossier §3.4: up > left > down > right).
/// LAB: Blinky's decision for J=(5,5) is made on entering (5,6) moving Up;
/// J's exits Left/Right tie in distance to his scatter target (5,-1)
/// (test tiles (4,5)/(6,5): dist_sq = 1+36 = 37 both) -> Left.
///
/// Hand-derived motion: ghost 75% speed = 240 subpx/tick, so a ghost steps
/// every tick except ticks 1, 17, 33... (accumulator starts empty):
/// pixels moved after t ticks = t - ceil(t/16). Blinky spawns at (44,60):
/// J center (44,44) reached at moved=16 (t=18); the Left turn applies on
/// the next step; by t=25 (moved 23) he is at (37,44) facing Left.
#[test]
fn tie_break_left_over_right() {
    let mut g = h::game_on(&h::lab_map(""), 1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 25, None);
    let b = h::ghost(&g, GhostId::Blinky);
    assert_eq!(b.dir, Dir::Left, "L/R tie resolves to Left");
    assert_eq!((b.pos.x.px(), b.pos.y.px()), (37, 44));
}

/// Tie-break Up over Down. LAB: Clyde (outside, at (28,44) facing Left)
/// decides for M=(1,5) on entering (2,5); M's exits Up/Down tie to his
/// scatter target (0,5) (test tiles (1,4)/(1,6): dist_sq = 1+1 = 2) -> Up.
/// Same 75% stepping: M center (12,44) at moved=16 (t=18); by t=25 he is
/// at (12,37) facing Up.
#[test]
fn tie_break_up_over_down() {
    let mut g = h::game_on(&h::lab_map(""), 1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 25, None);
    let c = h::ghost(&g, GhostId::Clyde);
    assert_eq!(c.dir, Dir::Up, "U/D tie resolves to Up");
    assert_eq!((c.pos.x.px(), c.pos.y.px()), (12, 37));
}

/// Red zone: a no-up tile removes Up from a scatter/chase ghost's choices
/// (dossier §3.7). With M=(1,5) marked no-up, Clyde's U/D tie must go Down.
#[test]
fn red_zone_bans_up_for_scatter_ghosts() {
    let mut g = h::game_on(&h::lab_map("[1, 5]"), 1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 25, None);
    let c = h::ghost(&g, GhostId::Clyde);
    assert_eq!(c.dir, Dir::Down, "banned Up leaves Down");
    assert_eq!((c.pos.x.px(), c.pos.y.px()), (12, 51));
}

/// Frightened ghosts ignore red zones (dossier §3.7; ROM mechanism in
/// supplements §12). RING with (1,5) marked no-up: after the energizer,
/// Blinky reverses and runs the loop clockwise; at (1,5), traveling Left,
/// the ONLY non-reverse exit is Up — banned for a scatter ghost (which
/// would dead-end), but the frightened chooser ignores the ban, so he
/// climbs col 1 (x=12, y<44) while still blue.
#[test]
fn frightened_ignores_red_zone() {
    let src = h::RING.replace("[spawns]", "no_up_tiles = [[1, 5]]\n[spawns]");
    let mut g = h::game_on(&src, 1);
    h::run_to_playing(&mut g);
    // Pac climbs col 1 (energizer eaten tick 32, see sim_movement) and
    // parks at the top-left corner.
    let mut saw_climb = false;
    for _ in 0..340 {
        g.tick(h::hold(Dir::Up));
        let b = h::ghost(&g, GhostId::Blinky);
        if b.frightened && b.state == GhostState::Active && b.pos.x.px() == 12 && b.pos.y.px() < 44
        {
            saw_climb = true;
            break;
        }
    }
    assert!(
        saw_climb,
        "frightened Blinky must climb through the banned corner"
    );
}

/// Eyes ignore red zones (supplements §12: dead ghosts never run the
/// red-zone test). Same map: pac descends after the energizer and parks at
/// (1,5); Blinky (clockwise, frightened) is eaten there; his eyes' only
/// route home climbs the banned corner — arrival (Entering, then revival)
/// proves the exemption.
#[test]
fn eyes_ignore_red_zone() {
    let src = h::RING.replace("[spawns]", "no_up_tiles = [[1, 5]]\n[spawns]");
    let mut g = h::game_on(&src, 1);
    h::run_to_playing(&mut g);
    // Climb to the energizer (eaten tick 32), then return to (12,44) and
    // park facing the wall.
    let _ = h::tick_n(&mut g, 40, Some(Dir::Up));
    let mut eaten_at = None;
    for t in 40..700u32 {
        let ev = g.tick(h::hold(Dir::Down));
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::GhostEaten {
                    ghost: GhostId::Blinky,
                    ..
                }
            )
        }) {
            eaten_at = Some(t);
            break;
        }
    }
    let eaten_at = eaten_at.expect("Blinky must be eaten at pac's (1,5) camp");
    let mut arrived = false;
    for _ in eaten_at..eaten_at + 500 {
        g.tick(h::hold(Dir::Down));
        let b = h::ghost(&g, GhostId::Blinky);
        if matches!(
            b.state,
            GhostState::Entering | GhostState::InHouse | GhostState::Leaving
        ) {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "eyes must cross the banned corner and reach the house"
    );
}

/// The Up-overflow bug affects only Pinky and Inky (direction users);
/// Blinky and Clyde ignore Pac-Man's facing entirely (dossier §3.6
/// "head-faking" note).
#[test]
fn blinky_and_clyde_ignore_pac_direction() {
    for d in Dir::IN_PRIORITY_ORDER {
        assert_eq!(
            chase_target(GhostId::Blinky, T(10, 10), d, T(5, 5), T(20, 20), T(25, -1)),
            T(10, 10)
        );
        assert_eq!(
            chase_target(GhostId::Clyde, T(10, 10), d, T(5, 5), T(20, 20), T(0, 34)),
            T(10, 10) // dist 10*sqrt(2) tiles >= 8
        );
    }
}

/// Eyes target the map's eyes tile (dossier §3.5; ROM (13,14) in
/// supplements §8), reported via `ghost_target`.
#[test]
fn eyes_target_is_house_door_tile() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    // Same chase scenario as the ring chain test: eat the (1,1) energizer,
    // chase right, eat Blinky.
    let mut eaten = false;
    for t in 0..400u32 {
        let dir = if t < 40 { Dir::Up } else { Dir::Right };
        let ev = g.tick(h::hold(dir));
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::GhostEaten {
                    ghost: GhostId::Blinky,
                    ..
                }
            )
        }) {
            eaten = true;
            break;
        }
    }
    assert!(eaten, "Blinky must be eaten in the ring chase");
    // After the score freeze he is Eyes and targets (5,1).
    let _ = h::tick_n(&mut g, 61, None);
    assert_eq!(h::ghost(&g, GhostId::Blinky).state, GhostState::Eyes);
    assert_eq!(g.ghost_target(GhostId::Blinky), Some(T(5, 1)));
}
