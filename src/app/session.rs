//! GameSession: the session loop as a deep module, tty-free.
//!
//! One play of one game — input sourcing, the fixed-tick accumulator,
//! recording, pause state, end conditions and the persistence policy — behind
//! a small interface ([`GameSession::on_key`] / [`GameSession::advance`] /
//! frame accessors / [`GameSession::finish`]). `play_once` in [`super`] is
//! wiring only: terminal events in, session stepped, frame out. Tests cross
//! the same seam the app does; nothing here touches a terminal or a clock.
//!
//! The live-vs-replay seam is [`InputSource`]: two adapters (live keyboard
//! steering, replay playback) behind one opaque interface. Before this seam
//! was named, the variation was spelled as four scattered `replay_src`
//! conditionals in `play_once` (seed selection, key filtering, per-tick
//! input, persistence policy); now each is one method of the source, and the
//! adapter is chosen exactly once, at construction.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crate::map::Map;
use crate::render::{Key, Overlay};
use crate::replay::Replay;
use crate::rules::Rules;
use crate::sim::Game;
use crate::types::{Difficulty, Dir, InputFrame, RenderState, TICK_HZ};

use super::persist::Store;

/// How one play ends (the session's terminal outcomes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayEnd {
    /// r while paused or on game over: same map/difficulty, fresh game.
    Restart,
    /// Esc while paused/on game over, or Enter dismissing game over.
    ToMenu,
    /// q / ctrl-c: quit the whole application.
    QuitApp,
}

/// Where per-tick [`InputFrame`]s come from: the seam between live play and
/// replay playback. Two adapters satisfy it — live keyboard steering
/// ([`InputSource::live`]) and recorded playback ([`InputSource::replay`]) —
/// and the choice is made once, at construction; no caller branches on the
/// kind again.
///
/// The interface is the four methods below; the representation is private.
#[derive(Debug, Clone)]
pub struct InputSource(Repr);

#[derive(Debug, Clone)]
enum Repr {
    Live {
        seed: u64,
        desired: Option<Dir>,
    },
    Replay {
        seed: u64,
        inputs: Vec<InputFrame>,
        pos: usize,
    },
}

impl InputSource {
    /// Live keyboard play. The caller picks the seed (the app uses the wall
    /// clock; tests pass a constant).
    pub fn live(seed: u64) -> InputSource {
        InputSource(Repr::Live {
            seed,
            desired: None,
        })
    }

    /// Playback of a recorded replay: seed and frames come from the
    /// recording; steering is ignored.
    pub fn replay(rp: &Replay) -> InputSource {
        InputSource(Repr::Replay {
            seed: rp.seed,
            inputs: rp.inputs.clone(),
            pos: 0,
        })
    }

    /// The PRNG seed this source dictates for the game it feeds.
    pub fn seed(&self) -> u64 {
        match self.0 {
            Repr::Live { seed, .. } | Repr::Replay { seed, .. } => seed,
        }
    }

    /// Hold a steering direction. Live play latches it into every following
    /// frame; replay playback ignores it (the recording already steered).
    pub fn steer(&mut self, dir: Dir) {
        if let Repr::Live { desired, .. } = &mut self.0 {
            *desired = Some(dir);
        }
    }

    /// The next tick's input frame. Live: the currently-held direction.
    /// Replay: the next recorded frame; an exhausted recording yields
    /// default (no-direction) frames forever.
    pub fn next_frame(&mut self) -> InputFrame {
        match &mut self.0 {
            Repr::Live { desired, .. } => InputFrame { dir: *desired },
            Repr::Replay { inputs, pos, .. } => {
                let f = inputs.get(*pos).copied().unwrap_or_default();
                *pos += 1;
                f
            }
        }
    }

    /// THE persistence policy, made explicit: replay sessions never persist.
    /// Replayed play is not the player's, so it must not pollute the score
    /// tables or overwrite a recording; only live sessions may write.
    pub fn persists(&self) -> bool {
        matches!(self.0, Repr::Live { .. })
    }
}

/// One play of one game, stepped by the caller.
///
/// Owns the [`Game`], the [`InputSource`], the fixed-tick accumulator (with
/// the long-stall clamp), the input recording, pause state and the end-key
/// semantics. Pause is the session's own state; game over is delegated to
/// [`Game::is_game_over`] (one owner, no mirrored flag). The caller's loop
/// is: feed keys to [`GameSession::on_key`], feed elapsed wall time to
/// [`GameSession::advance`], and recompose the frame from
/// [`GameSession::map`] / [`GameSession::render_state`] /
/// [`GameSession::overlay`] whenever `advance` returns true. When `on_key`
/// returns a [`PlayEnd`], call [`GameSession::finish`] to apply the
/// persistence policy, then stop.
#[derive(Debug)]
pub struct GameSession {
    game: Game,
    source: InputSource,
    recording: Replay,
    record_to: Option<PathBuf>,
    difficulty: Difficulty,
    /// High-score floor from the store at session start (HUD only).
    persisted_high: u32,
    paused: bool,
    /// An overlay flag flipped since the last `advance` (frame needs
    /// recomposing even without a tick).
    dirty: bool,
    /// Sim ticks run so far.
    ticks: u64,
    acc: Duration,
}

impl GameSession {
    /// Start one play: a fresh [`Game`] seeded by `source`, an empty
    /// recording, unpaused. `persisted_high` is the stored high score shown
    /// as the HUD floor; `record_to` is where [`GameSession::finish`] writes
    /// the recording (live sessions only).
    pub fn new(
        map: Map,
        difficulty: Difficulty,
        source: InputSource,
        record_to: Option<PathBuf>,
        persisted_high: u32,
    ) -> GameSession {
        let seed = source.seed();
        let recording = Replay::new(map.id().to_string(), difficulty, seed);
        GameSession {
            game: Game::new(map, Rules::classic(), difficulty, seed),
            source,
            recording,
            record_to,
            difficulty,
            persisted_high,
            paused: false,
            dirty: false,
            ticks: 0,
            acc: Duration::ZERO,
        }
    }

    /// Apply one key with the gameplay key semantics: arrows/WASD steer (via
    /// the source; replay ignores them), p toggles pause (gated off on game
    /// over), Esc pauses — or leaves to the menu when already paused or on
    /// game over — Enter dismisses game over, r restarts while paused or on
    /// game over, q quits. Returns the end the key produced, if any; the
    /// caller must then [`GameSession::finish`] the session.
    pub fn on_key(&mut self, key: Key) -> Option<PlayEnd> {
        let game_over = self.game.is_game_over();
        match key {
            Key::Up | Key::Down | Key::Left | Key::Right => {
                self.source.steer(match key {
                    Key::Up => Dir::Up,
                    Key::Down => Dir::Down,
                    Key::Left => Dir::Left,
                    _ => Dir::Right,
                });
                None
            }
            Key::Pause => {
                if !game_over {
                    self.paused = !self.paused;
                    self.dirty = true;
                }
                None
            }
            Key::Escape => {
                if game_over || self.paused {
                    return Some(PlayEnd::ToMenu);
                }
                self.paused = true;
                self.dirty = true;
                None
            }
            Key::Enter => game_over.then_some(PlayEnd::ToMenu),
            Key::Char('r') => (self.paused || game_over).then_some(PlayEnd::Restart),
            Key::Quit => Some(PlayEnd::QuitApp),
            _ => None,
        }
    }

    /// Advance wall time and run every sim tick that is due, recording each
    /// input frame. While paused or after game over, elapsed time is
    /// discarded (no backlog builds up); a long stall (> ~6 ticks) drops
    /// wall time instead of fast-forwarding the rules. Returns true when the
    /// composed frame changed: a tick ran, or a key flipped an overlay flag
    /// since the last call.
    pub fn advance(&mut self, dt: Duration) -> bool {
        let mut changed = std::mem::take(&mut self.dirty);
        let tick_len = Duration::from_secs_f64(1.0 / TICK_HZ);
        let max_backlog = tick_len * 6;
        if !self.paused && !self.game.is_game_over() {
            self.acc += dt;
            if self.acc > max_backlog {
                self.acc = max_backlog;
            }
            while self.acc >= tick_len && !self.game.is_game_over() {
                self.acc -= tick_len;
                let input = self.source.next_frame();
                self.game.tick(input);
                self.recording.push(input);
                self.ticks += 1;
                changed = true;
            }
        }
        changed
    }

    /// The map this session plays (for composing the frame).
    pub fn map(&self) -> &Map {
        self.game.map()
    }

    /// The sim's render snapshot with the persisted high-score floor folded
    /// into the HUD's high score.
    pub fn render_state(&self) -> RenderState {
        let mut rs = self.game.render_state();
        rs.high_score = rs.high_score.max(self.persisted_high).max(rs.score);
        rs
    }

    /// App-owned overlay flags for this frame (pause is session state;
    /// game over comes from the game).
    pub fn overlay(&self) -> Overlay {
        Overlay {
            paused: self.paused,
            game_over: self.game.is_game_over(),
        }
    }

    /// Sim ticks run so far (the recording's length).
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Finish the play: apply the persistence policy. Live sessions record
    /// a non-zero score (best-effort save) and write the recording to
    /// `record_to`; replay sessions persist nothing ([`InputSource::persists`]).
    pub fn finish(&self, store: &mut Store) -> io::Result<()> {
        if !self.source.persists() {
            return Ok(());
        }
        if self.game.score() > 0 {
            store.record_score(
                self.game.map().id(),
                self.difficulty,
                self.game.score(),
                self.game.level(),
            );
            let _ = store.save();
        }
        if let Some(p) = &self.record_to {
            fs::write(p, self.recording.to_text())?;
        }
        Ok(())
    }
}
