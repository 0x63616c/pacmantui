//! Scoring and collisions: the ghost-eat chain (dossier §5.1), the
//! tile-based death test with the pass-through bug (dossier §4.1), the
//! frightened proximity eat (supplements §12), fruit spawn/eat/expiry
//! (dossier §5.2, supplements §9), and the Hard-mode fruit exception
//! (tables.md A.2 prose; review finding F2, docs/plan/review-rules.md).

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::map::Map;
use pacmantui::rules::Rules;
use pacmantui::sim::Game;
use pacmantui::types::{Difficulty, Dir, Event, GhostId, InputFrame, Sequence};

/// RING with a dot-free bottom row: pac (12,44) faces Right, Blinky on the
/// same row facing Left. `threshold_scale = 0.001` rounds every scaled dot
/// threshold to 0 (docs/map-format.md), which in particular disables
/// Cruise Elroy (elroy1/2 trigger at <= 0 dots remaining — never), keeping
/// Blinky at the plain 75% needed by the derivations; the house ghosts
/// (limits 0) leave early but emerge onto the FAR top row.
fn passmap(blinky_offset: i32) -> String {
    h::pass_map(blinky_offset, "0.001", "#__________#")
}

/// The pass-through bug (dossier §4.1): if pac and a ghost swap tiles in
/// the same tick they never share one and no collision occurs. Hand-derived
/// with pac at 80% (1 px/tick) and Blinky at 75% (steps every tick except
/// 1, 17, 33...): pac x = 12+t, ghost x = 80 - (t - ceil(t/16)); tick 35
/// gives pac 47 (tile 5) / ghost 48 (tile 6) — 1 px apart across the
/// boundary — and tick 36 swaps them (pac 48/tile 6, ghost 47/tile 5).
/// This also proves death does NOT use the <4 px proximity rule: that rule
/// only applies to eating frightened ghosts (supplements §12).
#[test]
fn pass_through_tile_swap_is_survived() {
    let mut g = h::game_on(&passmap(-4), 1);
    h::run_to_playing(&mut g);
    let mut events = Vec::new();
    for _ in 0..35 {
        events.extend(g.tick(h::hold(Dir::Right)));
    }
    let (px, _) = h::pac_px(&g);
    let (bx, _) = h::ghost_px(&g, GhostId::Blinky);
    assert_eq!(
        (px, bx),
        (47, 48),
        "tick 35: 1 px apart across the boundary"
    );
    events.extend(g.tick(h::hold(Dir::Right)));
    let (px, _) = h::pac_px(&g);
    let (bx, _) = h::ghost_px(&g, GhostId::Blinky);
    assert_eq!((px, bx), (48, 47), "tick 36: tiles swapped");
    // Keep running: they separate, still alive.
    for _ in 0..24 {
        events.extend(g.tick(h::hold(Dir::Right)));
    }
    assert!(
        !events.iter().any(|e| matches!(e, Event::PacDying)),
        "pass-through must not kill: {events:?}"
    );
}

/// Control for the same geometry, 4 px later phase: with Blinky spawned at
/// x=84 the meeting lands both centers in tile 6 on tick 36 (pac 48, ghost
/// 51) — same tile ⇒ death (dossier §4.1), even though they never touched
/// pixel-wise.
#[test]
fn same_tile_collision_kills() {
    let mut g = h::game_on(&passmap(0), 1);
    h::run_to_playing(&mut g);
    let mut death_tick = None;
    for t in 1..=40u32 {
        let ev = g.tick(h::hold(Dir::Right));
        if ev.iter().any(|e| matches!(e, Event::PacDying)) {
            death_tick = Some(t);
            break;
        }
    }
    assert_eq!(death_tick, Some(36), "tile-sharing tick, hand-derived");
    assert!(matches!(g.sequence(), Sequence::DeathFreeze { .. }));
    let (px, _) = h::pac_px(&g);
    let (bx, _) = h::ghost_px(&g, GhostId::Blinky);
    assert_eq!((px, bx), (48, 51), "3 px apart but same tile (6,5)");
}

/// Eating a frightened ghost also triggers on per-axis pixel distance < 4
/// with DIFFERENT tiles (supplements §12: the second, energizer-active-only
/// test). Head-on meeting, hand-derived:
/// - pac spawns ON the energizer (eaten tick 1, then a 3-tick stop —
///   dossier §2.4), then moves right at fright-pac 90% = 288 subpx:
///   x = 12 + 1 + floor(288*(t-4)/256) for t >= 5, so x(26)=37, x(27)=38;
/// - Blinky (px 55, facing Right) is frightened from tick 1 (50% = 160
///   subpx, steps at floor(5t/8) increments): his first step (tick 2)
///   enters tile (7,5) and consumes the reversal, after which
///   x = 56 - (floor(5t/8) - 1): x(26) = x(27) = 41.
///
/// Tick 26 has dx=4 (no collision); tick 27 has pac 38 (tile 4) vs ghost
/// 41 (tile 5): dx=3 on different tiles ⇒ eaten by proximity alone.
#[test]
fn frightened_ghost_eaten_by_proximity() {
    let src = passmap(-4).replace("#__________#", "#o_________#").replace(
        "blinky = { tile = [10, 5], offset_px = [-4, 0], facing = \"left\" }",
        "blinky = { tile = [6, 5], offset_px = [3, 0], facing = \"right\" }",
    );
    let mut g = h::game_on(&src, 1);
    h::run_to_playing(&mut g);
    let ev = h::tick_n(&mut g, 26, Some(Dir::Right));
    assert!(
        !ev.iter().any(|e| matches!(e, Event::GhostEaten { .. })),
        "dx=4 at tick 26 is not yet a collision: {ev:?}"
    );
    let ev = g.tick(h::hold(Dir::Right));
    let eat = ev.iter().find(|e| matches!(e, Event::GhostEaten { .. }));
    match eat {
        Some(Event::GhostEaten {
            ghost,
            score,
            chain,
        }) => {
            assert_eq!((*ghost, *score, *chain), (GhostId::Blinky, 200, 1));
        }
        other => panic!("proximity eat on tick 27, got {other:?}"),
    }
    // Positions frozen at the collision configuration (supplements §1).
    let rs = g.render_state();
    let b = rs.ghosts[0];
    assert_eq!(rs.pac_pos.x.px(), 38);
    assert_eq!(b.pos.x.px(), 41);
    assert_ne!(b.pos.tile(), rs.pac_pos.tile(), "not a tile collision");
}

/// Full 200/400/800/1600 chain from one energizer (dossier §5.1), plus the
/// per-energizer reset: RING at threshold_scale 0.15 releases everyone
/// early; pac eats 15 dots + the (10,1) energizer and parks at the
/// top-right corner; the four frightened ghosts ride the forced loop into
/// him one at a time.
#[test]
fn ghost_chain_200_400_800_1600() {
    let mut g = h::game_on(&h::ring_with_scale("0.15"), 1);
    h::run_to_playing(&mut g);
    let mut eats = Vec::new();
    let mut pellets = 0u32;
    for _ in 0..1500u32 {
        let (x, y) = h::pac_px(&g);
        let dir = if pellets < 3 {
            Dir::Up
        } else if x == 12 && y < 44 {
            Dir::Down
        } else if y == 44 && x < 84 {
            Dir::Right
        } else {
            Dir::Up // climb col 10 through its dots into the energizer, park
        };
        for e in g.tick(InputFrame { dir: Some(dir) }) {
            match e {
                Event::GhostEaten {
                    ghost,
                    score,
                    chain,
                } => {
                    eats.push((ghost, score, chain));
                }
                Event::DotEaten { .. } | Event::EnergizerEaten { .. } => pellets += 1,
                _ => {}
            }
        }
        if eats.len() == 4 {
            break;
        }
    }
    let scores: Vec<u32> = eats.iter().map(|e| e.1).collect();
    let chains: Vec<u8> = eats.iter().map(|e| e.2).collect();
    assert_eq!(scores, vec![200, 400, 800, 1600], "{eats:?}");
    assert_eq!(chains, vec![1, 2, 3, 4]);
    let mut ghosts: Vec<GhostId> = eats.iter().map(|e| e.0).collect();
    ghosts.dedup();
    assert_eq!(ghosts.len(), 4, "four distinct ghosts: {eats:?}");
}

/// Fruit: spawns exactly when the (scaled) trigger count is reached, is
/// eaten on tile contact for the level's bonus points with a score popup
/// (dossier §5.2; scaling per docs/map-format.md). RING at scale 0.1:
/// trigger 1 = round(70*0.1) = 7 pellets; the fruit sits on (10,5), which
/// pac reaches two dots later (dot 9 and the fruit land on the same tick).
#[test]
fn fruit_spawns_on_trigger_and_is_eaten() {
    let mut g = h::game_on(&h::ring_with_scale("0.1"), 1);
    h::run_to_playing(&mut g);
    let mut pellets = 0u32;
    let mut spawned_at = None;
    let mut eaten = None;
    for _ in 0..200u32 {
        let ev = g.tick(h::hold(Dir::Right));
        for e in &ev {
            match e {
                Event::DotEaten { .. } => pellets += 1,
                Event::FruitSpawned { tile } => {
                    assert_eq!((tile.x, tile.y), (10, 5), "maps: ring fruit pos");
                    spawned_at = Some(pellets);
                }
                Event::FruitEaten { score } => eaten = Some((pellets, *score)),
                _ => {}
            }
        }
        if eaten.is_some() {
            break;
        }
    }
    assert_eq!(spawned_at, Some(7), "spawn on the 7th pellet exactly");
    // Dot 9 occupies the fruit tile: both are collected that tick.
    assert_eq!(eaten, Some((9, 100)), "L1 cherries = 100 (Table A.1)");
    let rs = g.render_state();
    assert!(rs.fruit.is_none(), "board fruit gone after eating");
    // POPUP_TICKS = 120, aged once by the same tick's housekeeping phase
    // (per-tick phase order in the sim module docs).
    assert_eq!(
        rs.popups,
        vec![(pacmantui::types::TilePos::new(10, 5), 100, 119)]
    );
}

/// Fruit expiry: the despawn window is 541..=600 ticks after spawn
/// (supplements §9: the ROM's 10-tick 1-second-clock task; drawn uniformly
/// from the same window — documented deviation in sim::timings). Played on
/// a modified classic map (threshold_scale 0.1 makes the trigger 7 dots)
/// where the spawn pocket stays ghost-free long enough.
#[test]
fn fruit_expires_within_the_rom_window() {
    let classic_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("maps/classic.pmtoml"),
    )
    .expect("shipped map readable");
    let src = format!("{classic_src}\n[rules]\nthreshold_scale = 0.1\n");
    let map = Map::parse(&src).expect("modified classic validates");
    // No RNG is consumed before the fruit draw (no frightened decisions
    // happen), so the duration is 541 + first_xorshift(seed) % 60; seed 19
    // draws 544 — the fruit (spawned on tick 63 by the 7th dot) expires on
    // tick 607, just before the ghosts reach the pocket where pac wiggles.
    let mut g = Game::new(map, Rules::classic(), Difficulty::Normal, 19);
    h::run_to_playing(&mut g);
    let mut spawn_tick = None;
    let mut expire_tick = None;
    for t in 1..1000u32 {
        let dir = if spawn_tick.is_none() || t.is_multiple_of(2) {
            Dir::Left
        } else {
            Dir::Right
        };
        let ev = g.tick(h::hold(dir));
        for e in &ev {
            match e {
                Event::FruitSpawned { .. } => spawn_tick = Some(t),
                Event::FruitExpired => expire_tick = Some(t),
                Event::PacDying => panic!("must survive until fruit expiry"),
                _ => {}
            }
        }
        if expire_tick.is_some() {
            break;
        }
    }
    let s = spawn_tick.expect("fruit spawned on the 7th dot");
    let x = expire_tick.expect("fruit expired");
    assert!(
        (541..=600).contains(&(x - s)),
        "expiry after {} ticks, ROM window 541-600",
        x - s
    );
}

/// Review finding F2 (docs/plan/review-rules.md; tables.md A.2 prose:
/// "bonus point values follow the displayed symbol, not the level"):
/// hard board 1 plays level-2 rules but shows and awards CHERRIES = 100,
/// not the Strawberry/300 of its effective gameplay row.
#[test]
fn hard_board_one_fruit_is_cherries_100() {
    let map = Map::parse(&h::ring_with_scale("0.1")).expect("ring validates");
    let mut g = Game::new(map, Rules::classic(), Difficulty::Hard, 1);
    h::run_to_playing(&mut g);
    assert_eq!(
        g.render_state().fruit_history,
        vec![0],
        "HUD shows cherries (index 0) on hard board 1"
    );
    let mut fruit_score = None;
    for _ in 0..300u32 {
        let ev = g.tick(h::hold(Dir::Right));
        if let Some(Event::FruitEaten { score }) =
            ev.iter().find(|e| matches!(e, Event::FruitEaten { .. }))
        {
            fruit_score = Some(*score);
            break;
        }
    }
    assert_eq!(
        fruit_score,
        Some(100),
        "cherries points, not strawberry 300"
    );
}
