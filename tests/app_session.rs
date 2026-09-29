//! GameSession/InputSource tests: the session loop exercised tty-free
//! through its own interface (the same seam `play_once` wires to the
//! terminal). Covers the live-vs-replay seam, pause, end conditions and the
//! persistence policy.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use pacmantui::app::persist::Store;
use pacmantui::app::session::{GameSession, InputSource, PlayEnd};
use pacmantui::map::Map;
use pacmantui::render::Key;
use pacmantui::replay::Replay;
use pacmantui::types::{Difficulty, Dir, InputFrame, TICK_HZ};

fn tick_len() -> Duration {
    Duration::from_secs_f64(1.0 / TICK_HZ)
}

/// Advance exactly `n` sim ticks (one tick of wall time per call, so the
/// backlog clamp never engages).
fn step_ticks(s: &mut GameSession, n: u64) {
    for _ in 0..n {
        s.advance(tick_len());
    }
}

fn live_session(seed: u64) -> GameSession {
    GameSession::new(
        Map::classic(),
        Difficulty::Normal,
        InputSource::live(seed),
        None,
        0,
    )
}

/// Fresh scratch dir per test (no env vars: tests run in parallel).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pacmantui-session-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn short_replay() -> Replay {
    let mut rp = Replay::new("classic".into(), Difficulty::Normal, 7);
    for d in [Some(Dir::Right), Some(Dir::Left), None] {
        rp.push(InputFrame { dir: d });
    }
    rp
}

#[test]
fn replay_source_yields_recorded_then_default_frames() {
    let rp = short_replay();
    let mut src = InputSource::replay(&rp);
    assert_eq!(src.seed(), 7);
    assert!(!src.persists());
    // Steering must not disturb playback.
    src.steer(Dir::Up);
    for want in &rp.inputs {
        assert_eq!(src.next_frame(), *want);
    }
    // Exhaustion yields default (no-direction) frames forever.
    for _ in 0..5 {
        assert_eq!(src.next_frame(), InputFrame::default());
    }
}

#[test]
fn live_source_latches_steering() {
    let mut src = InputSource::live(42);
    assert_eq!(src.seed(), 42);
    assert!(src.persists());
    assert_eq!(src.next_frame(), InputFrame::default());
    src.steer(Dir::Left);
    assert_eq!(
        src.next_frame(),
        InputFrame {
            dir: Some(Dir::Left)
        }
    );
    // Held until changed, exactly like the app's desired-direction latch.
    assert_eq!(
        src.next_frame(),
        InputFrame {
            dir: Some(Dir::Left)
        }
    );
    src.steer(Dir::Up);
    assert_eq!(src.next_frame(), InputFrame { dir: Some(Dir::Up) });
}

#[test]
fn session_keeps_ticking_past_replay_exhaustion() {
    let rp = short_replay();
    let mut s = GameSession::new(
        Map::classic(),
        Difficulty::Normal,
        InputSource::replay(&rp),
        None,
        0,
    );
    step_ticks(&mut s, 50);
    assert_eq!(s.ticks(), 50, "exhausted replay must not stall the session");
}

#[test]
fn pause_halts_sim_ticks_and_resumes() {
    let mut s = live_session(1);
    step_ticks(&mut s, 20);
    assert_eq!(s.ticks(), 20);

    assert_eq!(s.on_key(Key::Pause), None);
    assert!(s.overlay().paused);
    step_ticks(&mut s, 30);
    assert_eq!(s.ticks(), 20, "sim must not tick while paused");

    assert_eq!(s.on_key(Key::Pause), None);
    assert!(!s.overlay().paused);
    step_ticks(&mut s, 5);
    assert_eq!(s.ticks(), 25);
}

#[test]
fn escape_pauses_then_leaves_to_menu() {
    let mut s = live_session(1);
    assert_eq!(s.on_key(Key::Escape), None);
    assert!(s.overlay().paused);
    step_ticks(&mut s, 10);
    assert_eq!(s.ticks(), 0);
    assert_eq!(s.on_key(Key::Escape), Some(PlayEnd::ToMenu));
}

#[test]
fn restart_requires_pause_and_yields_restart_outcome() {
    let mut s = live_session(1);
    // Arrows and r during normal play end nothing.
    assert_eq!(s.on_key(Key::Left), None);
    assert_eq!(s.on_key(Key::Char('r')), None);
    assert_eq!(s.on_key(Key::Pause), None);
    assert_eq!(s.on_key(Key::Char('r')), Some(PlayEnd::Restart));
}

#[test]
fn quit_ends_immediately() {
    let mut s = live_session(1);
    assert_eq!(s.on_key(Key::Quit), Some(PlayEnd::QuitApp));
}

/// Drive an unsteered live game to game over (deterministic for a fixed
/// seed) and pin the game-over key gating.
#[test]
fn game_over_gates_pause_and_offers_dismiss_restart() {
    let mut s = live_session(1);
    let mut ticks = 0u64;
    while !s.overlay().game_over && ticks < 100_000 {
        step_ticks(&mut s, 1);
        ticks += 1;
    }
    assert!(
        s.overlay().game_over,
        "unsteered game should end in game over"
    );

    // Sim halted for good.
    let at_end = s.ticks();
    step_ticks(&mut s, 10);
    assert_eq!(s.ticks(), at_end);

    // Pause toggle is gated off on game over.
    assert_eq!(s.on_key(Key::Pause), None);
    assert!(!s.overlay().paused);

    // Enter dismisses, Esc leaves, r restarts.
    assert_eq!(s.on_key(Key::Enter), Some(PlayEnd::ToMenu));
    assert_eq!(s.on_key(Key::Escape), Some(PlayEnd::ToMenu));
    assert_eq!(s.on_key(Key::Char('r')), Some(PlayEnd::Restart));
}

#[test]
fn replay_session_never_persists() {
    let src = fs::read_to_string("docs/validation/replays/classic-level1-clear.replay").unwrap();
    let rp = Replay::from_text(&src).unwrap();
    let dir = scratch("replay-no-writes");
    let record_to = dir.join("out.replay");
    let mut store = Store::load(dir.clone());

    let mut s = GameSession::new(
        Map::classic(),
        rp.difficulty,
        InputSource::replay(&rp),
        Some(record_to.clone()),
        0,
    );
    // Run well past READY! so the replayed game has a real score.
    step_ticks(&mut s, 600);
    assert!(s.render_state().score > 0);

    assert_eq!(s.on_key(Key::Quit), Some(PlayEnd::QuitApp));
    s.finish(&mut store).unwrap();

    assert_eq!(store.high_score(s.map().id(), rp.difficulty), 0);
    assert!(
        !record_to.exists(),
        "replay sessions must not write recordings"
    );
    assert!(!dir.join("scores.toml").exists());
    assert!(!dir.join("settings.toml").exists());
}

#[test]
fn live_session_persists_score_and_recording() {
    let dir = scratch("live-persists");
    let record_to = dir.join("out.replay");
    let mut store = Store::load(dir.clone());

    let mut s = GameSession::new(
        Map::classic(),
        Difficulty::Normal,
        InputSource::live(99),
        Some(record_to.clone()),
        0,
    );
    s.on_key(Key::Left); // steer into the first dots once READY! ends
    let mut guard = 0;
    while s.render_state().score == 0 && guard < 2_000 {
        step_ticks(&mut s, 1);
        guard += 1;
    }
    let score = s.render_state().score;
    assert!(score > 0, "steered live game should eat a dot");

    assert_eq!(s.on_key(Key::Quit), Some(PlayEnd::QuitApp));
    s.finish(&mut store).unwrap();

    assert_eq!(store.high_score(s.map().id(), Difficulty::Normal), score);
    assert!(
        dir.join("scores.toml").exists(),
        "score table must be saved"
    );

    let written = Replay::from_text(&fs::read_to_string(&record_to).unwrap()).unwrap();
    assert_eq!(written.map_id, s.map().id());
    assert_eq!(written.seed, 99);
    assert_eq!(written.inputs.len() as u64, s.ticks());
}

#[test]
fn live_session_zero_score_skips_table_but_writes_recording() {
    let dir = scratch("live-zero");
    let record_to = dir.join("out.replay");
    let mut store = Store::load(dir.clone());

    let mut s = GameSession::new(
        Map::classic(),
        Difficulty::Normal,
        InputSource::live(5),
        Some(record_to.clone()),
        0,
    );
    step_ticks(&mut s, 10); // still in READY!, score 0
    assert_eq!(s.on_key(Key::Quit), Some(PlayEnd::QuitApp));
    s.finish(&mut store).unwrap();

    assert!(
        !dir.join("scores.toml").exists(),
        "zero scores are not recorded"
    );
    assert!(
        record_to.exists(),
        "live recordings are written regardless of score"
    );
}
