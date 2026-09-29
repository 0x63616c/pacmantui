//! Sequence timing to the tick: READY! (first-start vs subsequent), the
//! death sequence, the level-clear flash, the ghost-score freeze, plus
//! level progression (fruit history, extra life, no-fright level 17,
//! safe continuation past the classic kill screen).
//! Sources: arcade-supplements §1/§3/§4/§5 (ROM-derived values now in
//! sim::timings), dossier §4.2/§5.4, Table A.1 via `rules`.

#[path = "sim_helpers.rs"]
mod h;

use pacmantui::sim::Game;
use pacmantui::types::{Dir, Event, GhostId, GhostState, InputFrame, Sequence, TilePos};

/// First game start (supplements §4): two-phase READY! of 138 + 120 = 258
/// ticks; during phase A ("PLAYER ONE", no actors) neither Pac-Man nor the
/// ghosts are drawn; phase B shows everyone at their spawn points.
#[test]
fn first_ready_is_two_phase_258_ticks() {
    let mut g = h::classic_game(1);
    for k in 0..258u32 {
        assert!(matches!(g.sequence(), Sequence::Ready { tick } if tick == k));
        let rs = g.render_state();
        let visible = k >= 138;
        assert_eq!(rs.pac_visible, visible, "phase boundary at 138 (tick {k})");
        assert!(
            rs.ghosts.iter().all(|gr| gr.visible == visible),
            "ghosts hidden exactly while pac is (tick {k})"
        );
        assert!(rs.energizer_blink_on, "energizers steady during READY!");
        let ev = g.tick(InputFrame::default());
        assert!(ev.is_empty(), "READY! emits nothing: {ev:?}");
    }
    assert!(matches!(g.sequence(), Sequence::Playing));
}

/// Death sequence (supplements §3) on a pass-map with two dots and
/// `threshold_scale = 1.0`: with 18 pellets Blinky is Elroy 1 from the
/// start (scaled threshold 20 ≥ 18; Table A.1 L1: 80% = exactly 1
/// px/tick), so his position is x = 84 - t while pac (two dot-stops at
/// ticks 5 and 14) is at x = t + 10: tick 36 has pac 46/tile 5 vs ghost
/// 48/tile 6; tick 37 puts both at x=47 — same tile, death.
///
/// Then, to the tick: 60 freeze (all visible; housed ghosts keep bouncing),
/// 160 ghost-less death animation, LifeLost exactly 220 ticks after the
/// collision, 120-tick single-phase READY! with actors visible, pellets
/// preserved (dossier §4.2).
#[test]
fn death_sequence_timing_and_pellet_preservation() {
    let src = h::pass_map(0, "1.0", "#_.._______#");
    let mut g = h::game_on(&src, 1);
    h::run_to_playing(&mut g);
    let total = g.map().pellets_total();
    let mut death_tick = None;
    for t in 1..=60u32 {
        let ev = g.tick(h::hold(Dir::Right));
        if ev.iter().any(|e| matches!(e, Event::PacDying)) {
            death_tick = Some(t);
            break;
        }
    }
    assert_eq!(death_tick, Some(37), "hand-derived collision tick");
    assert_eq!(
        g.pellets_remaining(),
        total - 2,
        "both dots eaten pre-death"
    );

    // 60-tick freeze: everything halts in place but housed ghosts bounce
    // and remain visible (supplements §3).
    let pac_frozen = g.render_state().pac_pos;
    let inky_y0 = h::ghost(&g, GhostId::Inky).pos.y;
    let mut inky_moved = false;
    for k in 0..60u32 {
        assert!(matches!(g.sequence(), Sequence::DeathFreeze { tick } if tick == k));
        let rs = g.render_state();
        assert!(rs.pac_visible && rs.ghosts.iter().all(|x| x.visible));
        assert_eq!(rs.pac_pos, pac_frozen, "pac frozen in place");
        g.tick(InputFrame::default());
        if h::ghost(&g, GhostId::Inky).pos.y != inky_y0 {
            inky_moved = true;
        }
    }
    assert!(inky_moved, "housed ghosts keep bouncing through the freeze");

    // 160-tick death animation: ghosts gone, pac plays the animation.
    let mut life_lost_at = None;
    for k in 0..160u32 {
        assert!(matches!(g.sequence(), Sequence::DeathAnim { tick } if tick == k));
        let rs = g.render_state();
        assert!(rs.pac_visible, "pac alone on screen");
        assert!(rs.ghosts.iter().all(|x| !x.visible), "ghosts hidden");
        let ev = g.tick(InputFrame::default());
        if ev
            .iter()
            .any(|e| matches!(e, Event::LifeLost { lives_left: 2 }))
        {
            life_lost_at = Some(k);
        }
    }
    // Collision -> LifeLost = 60 + 160 = 220 ticks (supplements §3).
    assert_eq!(life_lost_at, Some(159), "LifeLost on the 220th tick");

    // Single-phase READY!: 120 ticks, actors visible at their spawns,
    // pellets preserved.
    for k in 0..120u32 {
        assert!(matches!(g.sequence(), Sequence::Ready { tick } if tick == k));
        let rs = g.render_state();
        assert!(rs.pac_visible && rs.ghosts.iter().all(|x| x.visible));
        g.tick(InputFrame::default());
    }
    assert!(matches!(g.sequence(), Sequence::Playing));
    let rs = g.render_state();
    assert_eq!(rs.lives, 2);
    assert_eq!(
        (rs.pac_pos.x.px(), rs.pac_pos.y.px()),
        (12, 44),
        "respawned"
    );
    assert_eq!(g.pellets_remaining(), total - 2, "eaten dots stay eaten");
}

/// Level clear (supplements §5): freeze 120 (everyone visible, static),
/// 4 white flashes at 12 ticks/phase (96), 18 blank — 234 ticks total with
/// ghosts erased from the first flash on and pac on screen throughout —
/// then the next level begins: board reset, schedule reset, fruit history
/// appended (dossier §5.4; sim module docs).
#[test]
fn level_clear_flash_timing_and_reset() {
    let mut g = h::game_on(h::SPRINT, 1);
    h::run_to_playing(&mut g);
    // Sprint level 1: pac (28,12) walks left at 80% (1 px/tick), eats the
    // energizer entering (1,1) at x=15 on tick 13, stops 3 ticks, then
    // returns right at the FRIGHTENED pac speed (Table A.1 L1: 90% = 288
    // subpx): x = 15 + floor(288*(t-16)/256); the (7,1) dot is entered at
    // x=56, i.e. floor(1.125*(t-16)) = 41 -> tick 53 clears the level.
    let mut cleared_at = None;
    for t in 1..=80u32 {
        let dir = if cleared_at.is_none() && t < 14 {
            Dir::Left
        } else {
            Dir::Right
        };
        let ev = g.tick(h::hold(dir));
        if ev
            .iter()
            .any(|e| matches!(e, Event::LevelCleared { level: 1 }))
        {
            cleared_at = Some(t);
            break;
        }
    }
    assert_eq!(cleared_at, Some(53), "hand-derived clear tick");
    let mut next_started = false;
    for k in 0..234u32 {
        assert!(matches!(g.sequence(), Sequence::LevelFlash { tick } if tick == k));
        let rs = g.render_state();
        assert!(rs.pac_visible, "pac stays on screen while the maze flashes");
        let ghosts_visible = k < 120;
        assert!(
            rs.ghosts.iter().all(|x| x.visible == ghosts_visible),
            "ghosts erased at the first flash (tick {k})"
        );
        let ev = g.tick(InputFrame::default());
        for e in ev {
            if matches!(e, Event::NextLevelStarted { level: 2 }) {
                next_started = true;
            }
        }
    }
    assert!(next_started, "NextLevelStarted after exactly 234 ticks");
    assert!(matches!(g.sequence(), Sequence::Ready { tick: 0 }));
    let rs = g.render_state();
    assert_eq!(rs.level, 2);
    assert_eq!(g.pellets_remaining(), 2, "board reset");
    assert_eq!(rs.fruit_history, vec![0, 1], "cherry then strawberry");
    assert_eq!(g.mode(), pacmantui::types::Mode::Scatter, "schedule reset");
}

/// Ghost-score freeze (supplements §1): 60 ticks in which Pac-Man is
/// hidden and both he and live ghosts are frozen in place, while the eyes
/// of PREVIOUSLY eaten ghosts keep moving and housed ghosts do NOT bounce.
/// Ring chase: Blinky is eaten first; during Pinky's later freeze Blinky
/// is still travelling as eyes.
#[test]
fn ghost_score_freeze_semantics() {
    let mut g = h::game_on(h::RING, 1);
    h::run_to_playing(&mut g);
    let mut second_eat = false;
    for t in 0..800u32 {
        let dir = if t < 40 { Dir::Up } else { Dir::Right };
        let ev = g.tick(h::hold(dir));
        if ev.iter().any(|e| {
            matches!(
                e,
                Event::GhostEaten {
                    ghost: GhostId::Pinky,
                    ..
                }
            )
        }) {
            second_eat = true;
            break;
        }
    }
    assert!(second_eat, "Pinky is the second ghost eaten in the chase");
    let pac0 = g.render_state().pac_pos;
    let inky0 = h::ghost(&g, GhostId::Inky).pos;
    let mut blinky_positions = Vec::new();
    for k in 0..60u32 {
        assert!(matches!(
            g.sequence(),
            Sequence::GhostScoreFreeze { ghost: GhostId::Pinky, score: 400, tick } if tick == k
        ));
        let rs = g.render_state();
        assert!(!rs.pac_visible, "pac hidden under the score sprite");
        assert_eq!(rs.pac_pos, pac0, "pac frozen");
        assert_eq!(
            h::ghost(&g, GhostId::Inky).pos,
            inky0,
            "house bounce paused"
        );
        blinky_positions.push(h::ghost(&g, GhostId::Blinky).pos);
        g.tick(InputFrame::default());
    }
    assert!(matches!(g.sequence(), Sequence::Playing));
    assert!(g.render_state().pac_visible);
    assert_eq!(h::ghost(&g, GhostId::Pinky).state, GhostState::Eyes);
    // Blinky (eaten earlier) kept moving as eyes/entering/leaving.
    assert!(
        blinky_positions.windows(2).any(|w| w[0] != w[1]),
        "the earlier victim's eyes keep moving through the freeze"
    );
}

/// Level progression on the sprint map through level 25:
/// - fruit history holds the last 7 Table A.1 symbols (dossier §5.2);
/// - level 17 has NO frightened mode (Table A.1 dashes: energizers only
///   reverse the ghosts — dossier §3.8), while 16 and 18 do;
/// - the ghosts still reverse on level 17's energizer;
/// - the extra life is awarded exactly once, at 10,000 points
///   (rules::extra_life_score; crossing happens on level 11's fruit:
///   10 levels × 60 + fruits 8800 + 50 + 3000 = 12,450);
/// - play continues past level 21 on the final table row (documented
///   deviation from the level-256 kill screen, sim module docs).
#[test]
fn progression_fruit_history_extra_life_and_level_17() {
    // Table A.1 bonus symbols as indices, levels 1..=21+ (cherry=0..key=7).
    let symbol = |level: u32| -> u8 {
        match level {
            1 => 0,
            2 => 1,
            3 | 4 => 2,
            5 | 6 => 3,
            7 | 8 => 4,
            9 | 10 => 5,
            11 | 12 => 6,
            _ => 7,
        }
    };
    let mut g = h::game_on(h::SPRINT, 1);
    let mut level = 1u32;
    let mut pellets_this_level = 0u32;
    let mut extra_lives = 0u32;
    let mut l17_fright_started = false;
    let mut l17_energizer_seen = false;
    let mut l17_reversal_seen = false;
    let mut l16_fright = false;
    // Blinky's direction at the START of each tick (the reversal can be
    // consumed within the energizer tick itself when a tile entry falls on
    // it, so the pre-energizer direction must come from the previous tick).
    let mut blinky_dir_before = Dir::Left;
    let mut pre_dir: Option<Dir> = None;
    for _ in 0..30_000u64 {
        let playing = matches!(g.sequence(), Sequence::Playing);
        let dir = if pellets_this_level == 0 {
            Dir::Left
        } else {
            Dir::Right
        };
        let before = blinky_dir_before;
        let ev = g.tick(InputFrame {
            dir: if playing { Some(dir) } else { None },
        });
        blinky_dir_before = g.render_state().ghosts[0].dir;
        for e in &ev {
            match e {
                Event::DotEaten { .. } => pellets_this_level += 1,
                Event::EnergizerEaten { .. } => {
                    pellets_this_level += 1;
                    if level == 17 {
                        l17_energizer_seen = true;
                        pre_dir = Some(before);
                    }
                }
                Event::FrightenedStarted => {
                    if level == 17 {
                        l17_fright_started = true;
                    }
                    if level == 16 {
                        l16_fright = true;
                    }
                }
                Event::ExtraLife => {
                    extra_lives += 1;
                    assert_eq!(level, 11, "threshold crossed on level 11's fruit");
                    assert!(g.score() >= 10_000);
                }
                Event::NextLevelStarted { level: l } => {
                    level = *l;
                    pellets_this_level = 0;
                    // History = last 7 level symbols, most recent last.
                    let first = level.saturating_sub(6).max(1);
                    let want: Vec<u8> = (first..=level).map(symbol).collect();
                    assert_eq!(g.render_state().fruit_history, want, "level {level}");
                }
                Event::PacDying | Event::GhostEaten { .. } => {
                    panic!("scenario must stay interference-free: {e:?}")
                }
                _ => {}
            }
        }
        // Level 17: watch for Blinky (Active throughout) reversing after
        // the energizer (the forced reversal still happens without
        // frightened time — dossier §3.8).
        if level == 17
            && let Some(pd) = pre_dir
        {
            assert!(
                g.render_state().fright_flash.is_none(),
                "no frightened display on level 17"
            );
            let b = g.render_state().ghosts[0];
            if b.state == GhostState::Active && b.dir == pd.opposite() {
                l17_reversal_seen = true;
            }
        }
        if level >= 25 {
            break;
        }
    }
    assert_eq!(
        level, 25,
        "progression continues past 21 (final row forever)"
    );
    assert!(l16_fright, "level 16 still has frightened time (Table A.1)");
    assert!(l17_energizer_seen);
    assert!(
        !l17_fright_started,
        "level 17: dashes in Table A.1 = no fright"
    );
    assert!(l17_reversal_seen, "ghosts still reverse on the energizer");
    assert_eq!(extra_lives, 1, "extra life exactly once");
    assert_eq!(g.lives(), 4);
    let _ = TilePos::new(0, 0);
    let _: Option<Game> = None;
}
