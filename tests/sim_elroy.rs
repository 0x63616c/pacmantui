//! Cruise Elroy (dossier §3.11): speed stages with custom-map threshold
//! scaling, the scatter-targeting override, and the post-death suspension.

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::types::{Dir, Event, GhostId, InputFrame, TilePos};

/// RING with `threshold_scale = 2.0`: scaled Elroy thresholds are
/// elroy1 = 40 and elroy2 = 20 dots remaining (docs/map-format.md
/// scaling; Table A.1 level 1: 20/10 at 80%/85%). With 25 pellets on the
/// board Blinky is Elroy 1 from the very start (25 <= 40) and becomes
/// Elroy 2 when the 5th pellet drops the remainder to 20.
///
/// Hand-derived speeds (Fix8 budgets):
/// - Elroy 1 = 80% = 256 subpx = exactly 1 px/tick: x = 84 - t while he
///   runs left along row 1 (x(16)=68, x(39)=45);
/// - pac (holding Right) eats row-5 dots on ticks 4,13,22,31,40 — Elroy 2
///   (85% = 272 subpx) starts inside tick 40 and delivers 17 px over the
///   next 16 ticks: x(55) = 45 - 17 = 28.
#[test]
fn elroy_stages_scaled_thresholds_and_speeds() {
    let mut g = h::game_on(&h::ring_with_scale("2.0"), 1);
    h::run_to_playing(&mut g);
    let bx = |g: &pacmantui::sim::Game| h::ghost(g, GhostId::Blinky).pos.x.px();
    let _ = h::tick_n(&mut g, 16, Some(Dir::Right));
    assert_eq!(bx(&g), 68, "Elroy 1: exactly 1 px/tick from the start");
    let _ = h::tick_n(&mut g, 23, Some(Dir::Right));
    assert_eq!(
        bx(&g),
        45,
        "still Elroy 1 through tick 39 (4 pellets eaten)"
    );
    let _ = h::tick_n(&mut g, 16, Some(Dir::Right));
    assert_eq!(bx(&g), 28, "Elroy 2 from the 5th pellet: 17 px in 16 ticks");
}

/// While Elroy, Blinky keeps chasing Pac-Man's tile during SCATTER (his
/// corner is abandoned), but the other ghosts scatter normally
/// (dossier §3.11).
#[test]
fn elroy_chases_during_scatter() {
    let mut g = h::game_on(&h::ring_with_scale("2.0"), 1);
    h::run_to_playing(&mut g);
    // Tick 50: Pinky has finished his 32-tick climb and is Active; the
    // schedule is still in scatter 1 (< 420).
    let _ = h::tick_n(&mut g, 50, Some(Dir::Right));
    assert_eq!(g.mode(), pacmantui::types::Mode::Scatter);
    let pac_tile = g.render_state().pac_pos.tile();
    assert_eq!(
        g.ghost_target(GhostId::Blinky),
        Some(pac_tile),
        "Elroy Blinky ignores his corner"
    );
    assert_eq!(
        g.ghost_target(GhostId::Pinky),
        Some(TilePos::new(0, -1)),
        "everyone else still scatters"
    );
}

/// After a life is lost, Elroy is suspended until Clyde moves to exit
/// (dossier §3.11): despite 16 pellets remaining (well under the scaled
/// elroy1 = 40), the respawned Blinky targets his scatter corner again and
/// moves at the plain 75% (15 px over the first 16 ticks; 240 subpx skips
/// tick 1).
#[test]
fn elroy_suspended_after_death() {
    let mut g = h::game_on(&h::ring_with_scale("2.0"), 1);
    h::run_to_playing(&mut g);
    // Pac eats the 9 row-5 dots and parks at the (10,5) corner; the
    // Elroy-2 Blinky loops around and kills him there.
    let mut died = false;
    for _ in 0..600u32 {
        let ev = g.tick(h::hold(Dir::Right));
        if ev.iter().any(|e| matches!(e, Event::PacDying)) {
            died = true;
            break;
        }
    }
    assert!(died, "Elroy Blinky must catch the parked pac");
    while !matches!(g.sequence(), pacmantui::types::Sequence::Playing) {
        g.tick(InputFrame::default());
    }
    assert!(
        g.pellets_remaining() <= 40,
        "Elroy range — only suspension hides it"
    );
    assert_eq!(
        g.ghost_target(GhostId::Blinky),
        Some(TilePos::new(11, -1)),
        "suspended: back to the scatter corner"
    );
    let x0 = h::ghost(&g, GhostId::Blinky).pos.x.px();
    let _ = h::tick_n(&mut g, 16, None);
    let x1 = h::ghost(&g, GhostId::Blinky).pos.x.px();
    assert_eq!(x0 - x1, 15, "suspended speed is the plain 75%");
}
