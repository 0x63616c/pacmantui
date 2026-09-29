//! Scatter/chase schedule boundaries, energizer reversal, and frightened
//! mode (dossier §3.2/§3.3/§3.8; durations from Table A.1 via `rules`).
//!
//! "Playing ticks" count only ticks where the sequence gate was `Playing`
//! (freezes like GhostScoreFreeze pause every gameplay timer —
//! supplements §1 — so schedule expectations are in playing ticks).

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::sim::Game;
use pacmantui::types::{Dir, Event, GhostId, InputFrame, Mode, Sequence};

/// Drive with a per-playing-tick input function; returns
/// (playing_tick, event) pairs. Ticks spent inside sequences do not count.
fn run_playing(
    g: &mut Game,
    total_playing: u32,
    mut input: impl FnMut(u32) -> Option<Dir>,
) -> Vec<(u32, Event)> {
    let mut out = Vec::new();
    let mut pt = 0u32;
    let mut guard = 0u32;
    while pt < total_playing {
        let playing = matches!(g.sequence(), Sequence::Playing);
        if playing {
            pt += 1;
        }
        let dir = if playing { input(pt) } else { None };
        for e in g.tick(InputFrame { dir }) {
            out.push((pt, e));
        }
        guard += 1;
        assert!(guard < 20 * total_playing + 10_000, "runaway loop");
    }
    out
}

/// Level 1 scatter 1 lasts exactly 7 s = 420 ticks (schedule table,
/// tables.md; rules stores ticks): the Scatter->Chase flip lands on playing
/// tick 421, not one earlier. Pac oscillates on the dot-free spawn tiles
/// (13/14, 26) so nothing perturbs the timer.
#[test]
fn scatter_one_flips_exactly_at_tick_421() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let ev = run_playing(&mut g, 421, |_| {
        let _ = 0;
        None // released stick: pac runs left and parks against the wall pocket
    });
    let flips: Vec<&(u32, Event)> = ev
        .iter()
        .filter(|(_, e)| matches!(e, Event::ModeChanged { .. }))
        .collect();
    assert_eq!(flips.len(), 1, "exactly one flip in 421 ticks: {flips:?}");
    assert_eq!(flips[0].0, 421, "flip on playing tick 421 exactly");
    assert!(matches!(
        flips[0].1,
        Event::ModeChanged { mode: Mode::Chase }
    ));
    assert_eq!(g.mode(), Mode::Chase);
}

/// One tick earlier there is no flip (boundary exactness).
#[test]
fn no_flip_at_tick_420() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let ev = run_playing(&mut g, 420, |_| None);
    assert!(
        !ev.iter()
            .any(|(_, e)| matches!(e, Event::ModeChanged { .. })),
        "scatter 1 must still be running at tick 420"
    );
    assert_eq!(g.mode(), Mode::Scatter);
}

/// Energizers force a reversal, consumed at the next tile entry (dossier
/// §3.3, §3.8). RING: pac's climb eats the energizer on tick 32 (see
/// sim_movement). Blinky derivation, hand-stepped from the Fix8 budgets:
/// - ticks 1..=31 at 75% (240 subpx): steps every tick except 1 and 17,
///   so x = 84 - 29 = 55, accumulator 16 subpx;
/// - from tick 32 he is frightened at 50% (160 subpx, Table A.1): steps on
///   ticks 33,35,36,38,39,41,43,44 -> x=48 after tick 43, and tick 44
///   crosses into tile (5,1) at x=47, consuming the queued reversal.
#[test]
fn energizer_reverses_ghosts_at_next_tile_entry() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    let _ = h::tick_n(&mut g, 43, Some(Dir::Up));
    let b = h::ghost(&g, GhostId::Blinky);
    assert_eq!(
        (b.pos.x.px(), b.dir),
        (48, Dir::Left),
        "tick 43: still leftbound"
    );
    assert!(b.frightened, "blue since tick 32");
    g.tick(h::hold(Dir::Up));
    let b = h::ghost(&g, GhostId::Blinky);
    assert_eq!(
        (b.pos.x.px(), b.dir),
        (47, Dir::Right),
        "tick 44 enters tile (5,1): reversal consumed, now facing Right"
    );
    // Ticks 45 (no step) and 46 (step right).
    let _ = h::tick_n(&mut g, 2, Some(Dir::Up));
    assert_eq!(h::ghost(&g, GhostId::Blinky).pos.x.px(), 48, "moving right");
}

/// Frightened facts on level 1, in one deterministic RING run:
/// - duration exactly 360 playing ticks (Table A.1: 6 s), reported through
///   `fright_flash: Some(..)`;
/// - exactly 5 white flash phases (Table A.1 "# of Flashes");
/// - the scatter/chase timer is FROZEN while frightened (dossier §3.2):
///   scatter 1 does NOT end at playing tick 421;
/// - ghosts do NOT reverse when frightened time expires (dossier §3.3).
///
/// Pac takes the right-hand route (9 row-5 dots, 3 col-10 dots, then the
/// (10,1) energizer) and parks at the top-RIGHT corner, away from the
/// house exit (which faces left). Hand-derived pellet cadence at 80% with
/// 1-tick dot stops: dots entered on playing ticks 4,13,22,31,40,49,58,67,
/// 76 (row 5), the Up turn at x=84 on tick 82, dots 86,95,104 (col 10),
/// and the ENERGIZER ON PLAYING TICK 113. Frightened ghosts are eaten on
/// contact (freezes pause all gameplay timers); revived ghosts kill the
/// parked pac only after the asserted window.
#[test]
fn fright_360_ticks_5_flashes_and_frozen_schedule() {
    const E: u32 = 113; // energizer playing tick (derived above)
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    let mut some_snapshots = 0u32;
    let mut white_groups = 0u32;
    let mut prev_white = false;
    let mut fright_end_pt = None;
    let mut first_flip_pt = None;
    let mut death_pt = None;
    let mut pt = 0u32;
    let mut dirs_around_end: Vec<(u32, Dir)> = Vec::new();
    let mut energizer_pt = None;
    for _ in 0..4000 {
        let playing = matches!(g.sequence(), Sequence::Playing);
        if playing {
            pt += 1;
        }
        // Driver: right along row 5, up col 10 from its corner, then park.
        let (x, _) = h::pac_px(&g);
        let dir = if x < 84 { Dir::Right } else { Dir::Up };
        let ev = g.tick(h::hold(dir));
        for e in &ev {
            match e {
                Event::EnergizerEaten { .. } => energizer_pt = Some(pt),
                Event::FrightenedEnded => fright_end_pt = Some(pt),
                Event::ModeChanged { .. } if first_flip_pt.is_none() => {
                    first_flip_pt = Some(pt);
                }
                Event::PacDying => death_pt = Some(pt),
                _ => {}
            }
        }
        if playing {
            let rs = g.render_state();
            match rs.fright_flash {
                Some(white) => {
                    some_snapshots += 1;
                    if white && !prev_white {
                        white_groups += 1;
                    }
                    prev_white = white;
                }
                None => prev_white = false,
            }
            if pt + 8 >= E + 360 && pt <= E + 364 {
                let b = h::ghost(&g, GhostId::Blinky);
                if b.state == pacmantui::types::GhostState::Active {
                    dirs_around_end.push((pt, b.dir));
                }
            }
        }
        if death_pt.is_some() {
            break;
        }
    }
    assert_eq!(energizer_pt, Some(E), "energizer on the derived tick");
    // Frightened lasts exactly 360 playing ticks (Table A.1: 6 s at L1).
    assert_eq!(fright_end_pt, Some(E + 360), "FrightenedEnded on E+360");
    assert_eq!(some_snapshots, 360, "exactly 360 frightened playing ticks");
    assert_eq!(white_groups, 5, "exactly 5 white flash phases (Table A.1)");
    // Schedule frozen while frightened (dossier §3.2): scatter 1 would have
    // flipped on playing tick 421 (< E+360); pac outlives that with no
    // ModeChanged ever emitted.
    let death = death_pt.expect("a revived ghost ends the scenario");
    assert!(
        death > 421,
        "must outlive the un-frozen flip point: {death}"
    );
    assert_eq!(
        first_flip_pt, None,
        "no Scatter->Chase flip: timer was frozen"
    );
    // No reversal at fright end (dossier §3.3): an Active ghost's facing
    // never flips 180 degrees across the boundary (corner turns are 90).
    for w in dirs_around_end.windows(2) {
        assert_ne!(
            w[1].1,
            w[0].1.opposite(),
            "no forced reversal at fright end: {dirs_around_end:?}"
        );
    }
}

/// A new energizer while frightened restarts the timer and re-frightens
/// the ghosts (dossier §3.8: chain resets per energizer; duration is per
/// energizer). RING chase scenario: energizer 1 tick 32, energizer 2 on
/// the far corner ~76 playing ticks later; FrightenedStarted fires twice.
#[test]
fn second_energizer_restarts_fright() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    let mut starts = 0;
    for t in 0..400u32 {
        let dir = if t < 40 { Dir::Up } else { Dir::Right };
        for e in g.tick(h::hold(dir)) {
            if matches!(e, Event::FrightenedStarted) {
                starts += 1;
            }
        }
    }
    assert_eq!(starts, 2, "both energizers frighten");
}
