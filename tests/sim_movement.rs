//! Movement exactness: speeds, eating pauses, buffered turns, cornering,
//! wall stops, and warp tunnels on both shipped maps.
//!
//! Every expectation is hand-derived from the docs cited per test — never
//! from running the sim. Key derivation facts:
//! - 100% speed = 1.25 px/tick (dossier §2.1); percent -> Fix8/tick via
//!   `Fix8::speed_from_percent` = round(320 * pct / 100) subpixels.
//!   Level 1 Pac-Man 80% (Table A.1) = 256 subpx = EXACTLY 1 px/tick.
//! - Eating a dot stops Pac-Man 1 tick, an energizer 3 (dossier §2.4).
//! - Classic Pac-Man spawns at pixel (112, 212) facing Left (dossier §1.3,
//!   maps/classic.pmtoml), i.e. occupied tile (14, 26); first dot leftward
//!   sits at (12, 26) (grid §1.2), entered when x reaches 103.

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::types::{Dir, Event, InputFrame, Sequence};

/// L1 Pac-Man moves exactly 1 px/tick at 80% (Table A.1; dossier §2.1).
/// The first 8 ticks of the classic game are dot-free: x goes 112 -> 104.
#[test]
fn pac_moves_exactly_one_px_per_tick_at_80_percent() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    assert_eq!(h::pac_px(&g), (112, 212));
    for t in 1..=8 {
        let ev = g.tick(h::hold(Dir::Left));
        assert_eq!(h::pac_px(&g), (112 - t, 212), "tick {t}");
        // Tick 1 releases Pinky (dot limit 0 — dossier §3.10, supplements
        // §10); nothing else may happen on the dot-free stretch.
        assert!(
            ev.iter().all(|e| matches!(e, Event::GhostReleased { .. })),
            "no eat/score events on the dot-free stretch: {ev:?}"
        );
    }
}

/// Dossier §2.4: eating a dot stops Pac-Man for exactly one tick.
/// Tick 9 enters tile (12,26) at x=103 and eats its dot; tick 10 is the
/// stop; tick 11 resumes at x=102.
#[test]
fn dot_eating_pauses_one_tick() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 8, Some(Dir::Left));
    let ev = g.tick(h::hold(Dir::Left));
    assert_eq!(h::pac_px(&g), (103, 212));
    assert!(
        ev.iter().any(|e| matches!(
            e,
            Event::DotEaten { tile, score: 10 } if tile.x == 12 && tile.y == 26
        )),
        "9th tick eats the (12,26) dot: {ev:?}"
    );
    g.tick(h::hold(Dir::Left));
    assert_eq!(h::pac_px(&g), (103, 212), "tick 10: stopped for the dot");
    g.tick(h::hold(Dir::Left));
    assert_eq!(h::pac_px(&g), (102, 212), "tick 11: moving again");
}

/// Dossier §2.4: an energizer stops Pac-Man for three ticks. On the RING
/// map Pac-Man climbs from (12,44): dots at (1,4)/(1,3)/(1,2) are entered
/// at y=39/31/23 (1-tick pause each), the energizer tile (1,1) at y=15,
/// followed by exactly 3 stopped ticks.
#[test]
fn energizer_eating_pauses_three_ticks() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    // y: 44 -> 39 takes 5 ticks (dot 1 entered at y=39 on tick 5), pause
    // tick 6; 8 px per further pellet + 1 pause: dot 2 at tick 14 (y=31),
    // pause 15; dot 3 at tick 23 (y=23), pause 24; energizer at tick 32
    // (y=15).
    let ev = h::tick_n(&mut g, 32, Some(Dir::Up));
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::EnergizerEaten { score: 50, .. })),
        "energizer eaten by tick 32: {ev:?}"
    );
    assert_eq!(h::pac_px(&g), (12, 15));
    for t in 33..=35 {
        g.tick(h::hold(Dir::Up));
        assert_eq!(h::pac_px(&g), (12, 15), "tick {t}: 3-tick energizer stop");
    }
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (12, 14), "tick 36: moving again");
}

/// Buffered pre-turn cornering (dossier §2.2/§2.3): holding Up, the turn at
/// (12,26) is taken at the first legal pixel. Pac-Man enters the turn tile
/// at x=103 (3 px from the engine's tile-center 100); during the corner he
/// moves 1 px up AND 1 px toward the centerline per step (45° cut at double
/// speed) until aligned at x=100.
#[test]
fn buffered_turn_corners_diagonally() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    // Ticks 1..=9 travel to x=103 (dot eaten on 9), tick 10 is the dot stop.
    let _ = h::tick_n(&mut g, 10, Some(Dir::Left));
    assert_eq!(h::pac_px(&g), (103, 212));
    // Hold Up: 3 diagonal steps, then straight up.
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (102, 211), "corner step 1 moves both axes");
    assert_eq!(
        g.render_state().pac_dir,
        Dir::Up,
        "orientation flips at once"
    );
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (101, 210), "corner step 2");
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (100, 209), "corner step 3: aligned on x=100");
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (100, 208), "straight up after the corner");
}

/// Pac-Man reverses direction freely at any time (dossier §2.3, used for
/// head-faking).
#[test]
fn reversal_is_instant() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 4, Some(Dir::Left));
    assert_eq!(h::pac_px(&g), (108, 212));
    g.tick(h::hold(Dir::Right));
    assert_eq!(h::pac_px(&g), (109, 212), "immediate mid-tile reversal");
    assert_eq!(g.render_state().pac_dir, Dir::Right);
}

/// Pac-Man stops at the tile center before a wall and his chomp animation
/// freezes while he is not actually moving (types::RenderState contract).
/// RING: the spawn (12,44) is the center of (1,5) with a wall at (0,5).
#[test]
fn pac_stops_at_walls_and_anim_freezes() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    let anim0 = g.render_state().pac_anim;
    for _ in 0..10 {
        g.tick(h::hold(Dir::Left));
        assert_eq!(h::pac_px(&g), (12, 44), "blocked by the wall at (0,5)");
        assert_eq!(
            g.render_state().pac_anim,
            anim0,
            "anim frozen while stopped"
        );
    }
    // Moving again advances the animation eventually (2 px per phase).
    let _ = h::tick_n(&mut g, 4, Some(Dir::Right));
    assert_ne!(
        g.render_state().pac_anim,
        anim0,
        "anim advances while moving"
    );
}

/// Classic tunnel warp (dossier §1.2: row 17 wraps col 0 <-> col 27;
/// docs/map-format.md "Tunnels and warping": pixel positions wrap modulo
/// the 224-px grid width). Pac-Man is steered to the left tunnel mouth and
/// through it; he is immune to the tunnel slowdown (dossier §2.5).
#[test]
fn warp_tunnel_classic() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    // Waypoint driver (inputs are not expectations): left to col 12, up to
    // row 23, left to col 6, up to row 17, then left through the tunnel.
    let mut wrapped = false;
    let mut prev_x = 112;
    for _ in 0..600 {
        let (x, y) = h::pac_px(&g);
        let dir = if y == 212 && x > 100 {
            Dir::Left
        } else if (97..=103).contains(&x) && y > 188 {
            Dir::Up
        } else if y == 188 && x > 52 {
            Dir::Left
        } else if (49..=55).contains(&x) && y > 140 {
            Dir::Up
        } else {
            Dir::Left
        };
        g.tick(h::hold(dir));
        let (nx, ny) = h::pac_px(&g);
        assert!(
            g.map().walkable(g.render_state().pac_pos.tile()),
            "pac must stay on walkable tiles (at {nx},{ny})"
        );
        if ny == 140 && prev_x <= 2 && nx >= 220 {
            // Left edge exit: x wraps modulo 224 (0 -> 223).
            assert_eq!(nx, prev_x + 223, "wrap re-enters at the right edge");
            wrapped = true;
            break;
        }
        prev_x = nx;
    }
    assert!(wrapped, "pac never warped through the classic tunnel");
}

/// Custom map ("Vertigo") vertical tunnel warp: col 3 wraps top <-> bottom
/// (maps/custom.pmtoml; grid is 26 rows = 208 px tall, so y wraps 0 -> 207).
#[test]
fn warp_tunnel_custom() {
    let mut g = h::custom_game(1);
    h::run_to_playing(&mut g);
    assert_eq!(
        h::pac_px(&g),
        (128, 60),
        "Vertigo spawn (maps/custom.pmtoml)"
    );
    let mut wrapped = false;
    let mut prev_y = 60;
    for _ in 0..600 {
        let (x, _) = h::pac_px(&g);
        // Left along row 7 to the col-3 tunnel, then up through the top edge.
        let dir = if x > 28 { Dir::Left } else { Dir::Up };
        g.tick(h::hold(dir));
        let (nx, ny) = h::pac_px(&g);
        assert!(
            g.map().walkable(g.render_state().pac_pos.tile()),
            "pac must stay on walkable tiles (at {nx},{ny})"
        );
        if prev_y <= 2 && ny >= 204 {
            assert_eq!(ny, prev_y + 207, "top-edge exit re-enters at the bottom");
            assert_eq!(nx, 28, "column preserved across the warp");
            wrapped = true;
            break;
        }
        prev_y = ny;
    }
    assert!(wrapped, "pac never warped through the Vertigo tunnel");
}

/// The turn window covers the whole turn tile: a post-turn (past-center)
/// corner is also taken, correcting BACK toward the centerline
/// (dossier §2.2 "post-turn"). Approaching (12,26) from the left moving
/// Right, Pac-Man is past x=100 when Up is first held.
#[test]
fn post_turn_corner_corrects_backwards() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    // Reach x=100 center of (12,26): 12 px left, one dot pause (tick 9's
    // dot at x=103) -> 13 ticks. Then overshoot: reverse and pass back
    // through the tile going Right to x=102 (2 px past center going right).
    let _ = h::tick_n(&mut g, 13, Some(Dir::Left));
    assert_eq!(h::pac_px(&g), (100, 212));
    let _ = h::tick_n(&mut g, 2, Some(Dir::Right));
    assert_eq!(h::pac_px(&g), (102, 212));
    // Hold Up: still inside (12,26); the corner moves up-left back to 100.
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (101, 211), "post-turn step 1");
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (100, 210), "post-turn step 2: aligned");
    g.tick(h::hold(Dir::Up));
    assert_eq!(h::pac_px(&g), (100, 209), "straight up");
}

/// The READY! gate: input during READY is ignored and the sim stays put
/// (control begins only when the sequence ends — supplements §4).
#[test]
fn input_ignored_during_ready() {
    let mut g = h::classic_game(1);
    for _ in 0..50 {
        g.tick(h::hold(Dir::Left));
    }
    assert!(matches!(g.sequence(), Sequence::Ready { tick: 50 }));
    assert_eq!(h::pac_px(&g), (112, 212), "no movement during READY");
    let _ = InputFrame::default();
}
