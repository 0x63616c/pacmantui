//! Ghost-house mechanics: personal dot counters, the post-death global
//! counter with the Clyde-deactivation quirk, the no-dot force-release
//! timer, bounce/exit motion, and the eyes round trip.
//! Sources: dossier §3.9/§3.10, arcade-supplements §8/§10; scaling per
//! docs/map-format.md "Rule adaptations".

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::types::{Dir, Event, GhostId, GhostState, InputFrame, Sequence};

/// Wiggle input that keeps classic pac on his dot-free spawn tiles.
fn wiggle(t: u32) -> Option<Dir> {
    Some(if t.is_multiple_of(2) {
        Dir::Left
    } else {
        Dir::Right
    })
}

/// Pinky's dot limit is 0 on every level (dossier §3.10): released on the
/// first playing tick; his climb out (24 px at 0.5 px/tick, supplements
/// §10) puts him at the outside-door point (112,116), facing Left, Active,
/// exactly 48 ticks after control starts.
#[test]
fn pinky_releases_immediately_and_exits_in_48_ticks() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let ev = g.tick(InputFrame { dir: wiggle(0) });
    assert!(
        ev.iter().any(|e| matches!(
            e,
            Event::GhostReleased {
                ghost: GhostId::Pinky
            }
        )),
        "Pinky released on playing tick 1: {ev:?}"
    );
    for t in 1..47 {
        g.tick(InputFrame { dir: wiggle(t) });
        assert_eq!(h::ghost(&g, GhostId::Pinky).state, GhostState::Leaving);
    }
    g.tick(InputFrame { dir: wiggle(47) });
    let p = h::ghost(&g, GhostId::Pinky);
    assert_eq!(p.state, GhostState::Active, "Active on tick 48");
    assert_eq!((p.pos.x.px(), p.pos.y.px()), (112, 116));
    assert_eq!(p.dir, Dir::Left, "exits facing Left (supplements §7/§10)");
}

/// Level-1 personal limits: Inky 30, Clyde 60 (dossier §3.10) — the
/// release event lands on the very tick of the 30th/60th pellet. Pac is
/// waypoint-driven through a 60-dot sweep of the classic maze's lower half.
#[test]
fn classic_level1_inky_at_30_dots_clyde_at_60() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let mut pellets = 0u32;
    let mut inky_at = None;
    let mut clyde_at = None;
    for _ in 0..1400u32 {
        let (x, y) = h::pac_px(&g);
        // Waypoint phases (inputs, not expectations).
        let dir = if y == 212 && x > 52 && clyde_at.is_none() && pellets < 20 {
            Dir::Left
        } else if x == 52 && y < 236 {
            Dir::Down
        } else if y == 236 && x > 12 && pellets < 18 {
            Dir::Left
        } else if x == 12 && y < 260 {
            Dir::Down
        } else if y == 260 && x < 212 {
            Dir::Right
        } else if x == 212 && y > 236 {
            Dir::Up
        } else if y == 236 && x > 172 {
            Dir::Left
        } else if x == 172 && y > 212 {
            Dir::Up
        } else {
            Dir::Left // final leg along row 26; parks at the 60th pellet
        };
        let ev = g.tick(InputFrame { dir: Some(dir) });
        let mut ate = false;
        for e in &ev {
            match e {
                Event::DotEaten { .. } | Event::EnergizerEaten { .. } => {
                    pellets += 1;
                    ate = true;
                }
                Event::GhostReleased {
                    ghost: GhostId::Inky,
                } => inky_at = Some(pellets),
                Event::GhostReleased {
                    ghost: GhostId::Clyde,
                } => clyde_at = Some(pellets),
                Event::PacDying => panic!("pac must survive the sweep"),
                _ => {}
            }
        }
        // Release evaluation runs in the same tick as the pellet (phase
        // order in sim module docs), so correlating with the running count
        // is exact when both appear in one tick's events.
        let _ = ate;
        if clyde_at.is_some() {
            break;
        }
    }
    assert_eq!(inky_at, Some(30), "Inky leaves with the 30th pellet");
    assert_eq!(clyde_at, Some(60), "Clyde leaves with the 60th pellet");
}

/// No-dot force release (dossier §3.10): with no dots eaten at all, the
/// most-preferred housed ghost is force-released every 4 s (level 1-4:
/// 240 ticks): Inky on playing tick 240, Clyde on 480.
#[test]
fn no_dot_timer_force_releases_every_240_ticks() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let mut releases = Vec::new();
    for t in 0..500u32 {
        for e in g.tick(InputFrame { dir: wiggle(t) }) {
            if let Event::GhostReleased { ghost } = e {
                releases.push((t + 1, ghost));
            }
        }
    }
    assert_eq!(
        releases,
        vec![
            (1, GhostId::Pinky),
            (240, GhostId::Inky),
            (480, GhostId::Clyde),
        ]
    );
}

/// In-house bounce: 0.5 px/tick, ±4 px around the slot, full cycle 32
/// ticks (supplements §10). Classic Inky starts at (96,140) facing Up.
#[test]
fn house_bounce_amplitude_and_period() {
    let mut g = h::classic_game(1);
    h::run_to_playing(&mut g);
    let y = |g: &pacmantui::sim::Game| h::ghost(g, GhostId::Inky).pos.y.px();
    let mut track = Vec::new();
    for t in 0..48u32 {
        g.tick(InputFrame { dir: wiggle(t) });
        track.push(y(&g));
    }
    // Steps land on even ticks: top of the swing (136) after 8 ticks,
    // bottom (144) after 24, top again after 40.
    assert_eq!(track[7], 136, "top of swing at tick 8");
    assert_eq!(track[23], 144, "bottom of swing at tick 24");
    assert_eq!(track[39], 136, "32-tick full cycle");
    assert!(track.iter().all(|&v| (136..=144).contains(&v)), "{track:?}");
}

/// Global counter after a death (dossier §3.10) on the RING map with
/// `threshold_scale = 0.15` (docs/map-format.md: scaled = round(v * s)):
/// personal limits Pinky 0 / Inky round(4.5)=5 / Clyde round(9)=9;
/// global limits Pinky round(1.05)=1 / Inky round(2.55)=3 / Clyde
/// round(4.8)=5.
///
/// Pre-death pac eats exactly 12 pellets (3 on col 1, 9 on row 5), so
/// Clyde's personal counter is 7 (pellets 6..12, after Inky leaves at 5)
/// when Blinky's loop sweep kills the pac parked at the (10,5) corner.
/// After the death the global counter releases Pinky at pellet 1 and Inky
/// at 3; at 5 Clyde is housed, so the counter is RESET AND DEACTIVATED
/// with no release (the quirk); the preserved personal counter then frees
/// Clyde at post-death pellet 7 (7+2 = 9 = limit). Had the counter stayed
/// active, no dot-based release could ever free him (dossier §3.10's
/// documented trick).
#[test]
fn global_counter_7_17_32_with_clyde_deactivation_quirk() {
    let mut g = h::game_on(&h::ring_with_scale("0.15"), 1);
    h::run_to_playing(&mut g);

    // --- pre-death: 3 col-1 dots, then 9 row-5 dots; park at (10,5) ---
    let mut pellets = 0u32;
    let mut inky_at = None;
    let mut died = false;
    for _ in 0..1000u32 {
        let (x, y) = h::pac_px(&g);
        let dir = if pellets < 3 {
            Dir::Up
        } else if y < 44 && x == 12 {
            Dir::Down
        } else {
            Dir::Right // along row 5; blocked at the corner center (84,44)
        };
        let ev = g.tick(InputFrame { dir: Some(dir) });
        for e in &ev {
            match e {
                Event::DotEaten { .. } | Event::EnergizerEaten { .. } => pellets += 1,
                Event::GhostReleased {
                    ghost: GhostId::Inky,
                } => inky_at = Some(pellets),
                Event::GhostReleased {
                    ghost: GhostId::Clyde,
                } => {
                    panic!("Clyde must stay housed pre-death (counter 7 < 9)")
                }
                Event::PacDying => died = true,
                _ => {}
            }
        }
        if died {
            break;
        }
    }
    assert!(died, "Blinky must sweep into the parked pac");
    assert_eq!(pellets, 12, "exactly 12 pellets pre-death");
    assert_eq!(inky_at, Some(5), "scaled personal limit: Inky at pellet 5");

    // --- post-death: global counter path ---
    while !matches!(g.sequence(), Sequence::Playing) {
        g.tick(InputFrame::default());
    }
    let mut post = 0u32;
    let mut first: Vec<(u32, GhostId)> = Vec::new();
    for _ in 0..1500u32 {
        let (x, y) = h::pac_px(&g);
        // Climb the (now dot-free) col 1 to the energizer, then right
        // along row 1. Frightened ghosts met head-on are eaten (freezes,
        // not deaths) and may be re-housed and re-released — only each
        // ghost's FIRST release carries the counter semantics under test.
        let dir = if x == 12 && y > 12 {
            Dir::Up
        } else {
            Dir::Right
        };
        let ev = g.tick(InputFrame { dir: Some(dir) });
        for e in &ev {
            match e {
                Event::DotEaten { .. } | Event::EnergizerEaten { .. } => post += 1,
                Event::GhostReleased { ghost } => {
                    if !first.iter().any(|r| r.1 == *ghost) {
                        first.push((post, *ghost));
                    }
                }
                Event::PacDying => panic!("no second death inside the window"),
                _ => {}
            }
        }
        if first.iter().any(|r| r.1 == GhostId::Clyde) || post >= 9 {
            break;
        }
    }
    assert_eq!(
        first,
        vec![
            (1, GhostId::Pinky), // global == 1 (scaled 7)
            (3, GhostId::Inky),  // global == 3 (scaled 17)
            // pellet 5 (scaled 32): Clyde housed -> deactivate, NO release
            (7, GhostId::Clyde), // personal counter 7+2 reaches the limit 9
        ]
    );
}

/// Eyes round trip (dossier §3.8; supplements §2/§8): eaten ghost's eyes
/// move at exactly 2 px/tick, navigate to the outside-door point, descend
/// through the door at 2 px/tick, revive in the house, and re-exit
/// (Blinky's limit is 0: he leaves immediately once revived).
#[test]
fn eyes_full_round_trip() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    // Ring chase: energizer at (1,1), pursue right; Blinky is eaten.
    let mut eaten = false;
    for t in 0..400u32 {
        let dir = if t < 40 { Dir::Up } else { Dir::Right };
        if g.tick(h::hold(dir)).iter().any(|e| {
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
    assert!(eaten);
    // Ghost-score freeze holds for 60 ticks (supplements §1), then Eyes.
    for _ in 0..60 {
        assert!(matches!(g.sequence(), Sequence::GhostScoreFreeze { .. }));
        g.tick(InputFrame::default());
    }
    assert!(matches!(g.sequence(), Sequence::Playing));
    assert_eq!(h::ghost(&g, GhostId::Blinky).state, GhostState::Eyes);
    // Eyes travel at exactly 2 px/tick (supplements §2).
    let p0 = h::ghost_px(&g, GhostId::Blinky);
    g.tick(InputFrame::default());
    let p1 = h::ghost_px(&g, GhostId::Blinky);
    let d = (p1.0 - p0.0).abs() + (p1.1 - p0.1).abs();
    assert_eq!(d, 2, "eyes cover 2 px per tick");
    // Full trip: Entering (descends on the house-center column x=48),
    // InHouse (revived), Leaving, Active again.
    let mut seen = vec![GhostState::Eyes];
    for _ in 0..400 {
        g.tick(InputFrame::default());
        let b = h::ghost(&g, GhostId::Blinky);
        if *seen.last().unwrap() != b.state {
            seen.push(b.state);
        }
        if b.state == GhostState::Entering {
            assert_eq!(b.pos.x.px(), 48, "descends through the door column");
        }
        if seen.last() == Some(&GhostState::Active) && seen.len() > 1 {
            break;
        }
    }
    assert_eq!(
        seen,
        vec![
            GhostState::Eyes,
            GhostState::Entering,
            GhostState::InHouse,
            GhostState::Leaving,
            GhostState::Active,
        ],
        "full revival cycle in order"
    );
}

/// Custom-map ("Vertigo", 340 pellets) threshold scaling applies to the
/// post-death global counter too: the Pinky release lands on the 10th
/// pellet — round(7 * 340/244) = round(9.75) = 10 — instead of the classic
/// 7 (docs/map-format.md "Rule adaptations"; dossier §3.10).
#[test]
fn vertigo_scales_global_counter_thresholds() {
    let mut g = h::custom_game(1);
    h::run_to_playing(&mut g);
    // Idle: pac auto-runs left along row 7, eats its dots, parks at the
    // wall; Blinky eventually kills him (all deterministic, no RNG use —
    // no frightened decisions happen before the death).
    let mut died = false;
    for _ in 0..5000u32 {
        let ev = g.tick(InputFrame::default());
        if ev.iter().any(|e| matches!(e, Event::PacDying)) {
            died = true;
            break;
        }
    }
    assert!(died, "Blinky must reach the parked pac");
    while !matches!(g.sequence(), Sequence::Playing) {
        g.tick(InputFrame::default());
    }
    // Post-death: eat rightward along row 7; Pinky waits for the SCALED
    // global count.
    let mut post = 0u32;
    let mut pinky_at = None;
    for _ in 0..600u32 {
        let ev = g.tick(h::hold(Dir::Right));
        for e in &ev {
            match e {
                Event::DotEaten { .. } | Event::EnergizerEaten { .. } => post += 1,
                Event::GhostReleased {
                    ghost: GhostId::Pinky,
                } => pinky_at = Some(post),
                _ => {}
            }
        }
        if pinky_at.is_some() {
            break;
        }
    }
    assert_eq!(pinky_at, Some(10), "scaled global limit, not the classic 7");
}
