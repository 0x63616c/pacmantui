//! Deterministic game simulation (the arcade rules engine).
//!
//! Pure: no I/O, no clock, no terminal. One `tick` = one arcade frame.
//! Rule sources: docs/research/dossier-mechanics.md (cited as "dossier §N"),
//! tables via `rules`, and ROM-derived supplemental values in [`timings`]
//! (cited as "supplements §N" = docs/research/arcade-supplements.md).
//!
//! # Per-tick phase order (while `Sequence::Playing`)
//!
//! Boundary-tested by `tests/sim_modes.rs` / `tests/sim_sequences.rs`:
//!
//! 1.  **Mode timers** — frightened countdown (scatter/chase timer frozen
//!     while frightened, dossier §3.2), else scatter/chase phase advance;
//!     flips emit [`Event::ModeChanged`] and queue reversals (dossier §3.3).
//! 2.  **Pac-Man movement** — eating-pause countdown, buffered turns,
//!     cornering, warp (dossier §2.2–§2.5).
//! 3.  **Pellet eating** at Pac-Man's tile — scoring, eating pauses, house
//!     dot counters, fruit triggers, energizer effects (fright + reversal).
//! 4.  **Fruit** — eat check, then despawn countdown.
//! 5.  **House releases** — personal-counter / limit-zero release of the
//!     most-preferred housed ghost (dossier §3.10); at most one per tick.
//! 6.  **Ghost movement** — bounce, leaving, active decisions, eyes,
//!     entering/revival (dossier §3.4–§3.9).
//! 7.  **No-dot force-release timer** (dossier §3.10).
//! 8.  **Collisions** — tile test for death; tile-or-proximity (<4 px both
//!     axes) for eating a frightened ghost (dossier §4.1, supplements §12).
//!     Checked once, after both actors moved, preserving the pass-through
//!     (tile swap) bug.
//! 9.  **Level-clear check** (skipped if a collision started a sequence; a
//!     simultaneous death wins).
//! 10. **Housekeeping** — extra life, popups, animation/blink counters,
//!     high score.
//!
//! # RNG
//!
//! Deterministic xorshift32 ([`Rng`]); replays store the seed. Consumption
//! points (the only two, in phase order within a tick):
//!
//! 1. Frightened ghost turn choice, one `next_u32` per frightened decision
//!    (ghost order Blinky, Pinky, Inky, Clyde within a tick) — dossier §3.8.
//! 2. Fruit display duration, one `next_u32` per fruit spawn — supplements
//!    §9 (documented deviation: the ROM's 541–600-frame spread comes from
//!    clock-phase aliasing; a uniform RNG draw over the same window is
//!    distribution-equivalent).
//!
//! The RNG is reseeded from the game seed at every level start and after
//! every life lost (dossier §3.8: "reset with the same initial seed value at
//! the start of each new level and whenever a life is lost").
//!
//! # Documented deviations from the arcade
//!
//! - Level counter is a `u32`; play continues past level 255 using the
//!   Table A.1 "21+" row forever instead of reproducing the level-256 kill
//!   screen (dossier §6 "Level 256", deliberate per IMPLEMENTATION.md).
//! - Fruit duration is drawn uniformly from the ROM's 541–600-frame window
//!   instead of emulating the global 60-frame clock (supplements §9).
//! - Cornering window: with this engine's tile centers at pixel offset 4
//!   (`types::PxPos::tile_center`), the pre/post-turn split measured by the
//!   dossier as 3/4 px (center at offset 3) becomes 4/3 px — same window
//!   size, shifted one pixel by the center convention (dossier §2.2).
//! - House releases are staggered at most one ghost per tick; on levels
//!   where several limits are zero the followers leave on successive ticks.
//! - On Hard difficulty the fruit symbol/score follow the *board* number,
//!   not the (higher) effective gameplay level — the bonus fields are read
//!   from `rules.level_spec(level, Difficulty::Normal)` (tables.md A.2
//!   prose; review finding F2 in docs/plan/review-rules.md).
//!
//! Determinism: fixed arrays (no map iteration), integer math only.
//!
//! Owner: W1-SIM agent. Public signatures are the contract; extend, don't break.

pub mod timings;

mod actors;

pub use actors::chase_target;

use crate::map::{Cell, Map};
use crate::rules::{BonusSymbol, Rules};
use crate::types::{
    Difficulty, Dir, Event, Fix8, GhostId, GhostRender, GhostState, InputFrame, Mode, PxPos,
    RenderState, Sequence, TilePos,
};

/// Dot score (dossier §5.1).
const DOT_POINTS: u32 = 10;
/// Energizer score (dossier §5.1).
const ENERGIZER_POINTS: u32 = 50;
/// HUD fruit-history length: current + last six rounds (dossier §5.2).
const FRUIT_HISTORY_LEN: usize = 7;

/// Pac-Man's mutable state.
#[derive(Debug, Clone)]
struct Pac {
    pos: PxPos,
    dir: Dir,
    /// Fix8 movement budget (sub-pixel accumulator).
    accum: i32,
    /// Eating-pause ticks left (dossier §2.4).
    pause: u32,
    /// Cornering: pixels left to the new path's centerline; 0 = not cornering.
    corner_left: i32,
    /// Cornering: per-step correction (unit px) toward the centerline.
    corner_step: (i32, i32),
    /// Total pixels moved (drives the chomp animation; advances only while
    /// actually moving).
    anim_px: u32,
}

/// One ghost's mutable state. Indexed by `GhostId::ALL` order.
#[derive(Debug, Clone)]
struct Ghost {
    state: GhostState,
    pos: PxPos,
    dir: Dir,
    /// Direction to take at the center of the currently occupied tile
    /// (decided when entering the previous tile — dossier §3.4).
    next_dir: Dir,
    /// Direction to take at the center of the next tile (decided on entering
    /// the current tile — dossier §3.4 "looks one tile ahead").
    future_dir: Dir,
    accum: i32,
    frightened: bool,
    /// Forced reversal queued by a mode flip / energizer; consumed at the
    /// next tile entry (dossier §3.3).
    reverse_pending: bool,
    /// Exit the house moving right instead of left (a mode change happened
    /// while housed — dossier §3.9).
    exit_right: bool,
    /// Personal house dot counter (dossier §3.10).
    dot_counter: u32,
}

#[derive(Debug, Clone)]
pub struct Game {
    map: Map,
    rules: Rules,
    difficulty: Difficulty,
    seed: u64,
    rng: Rng,

    level: u32,
    score: u32,
    high_score: u32,
    lives: u8,
    extra_life_awarded: bool,
    game_over: bool,
    fruit_history: Vec<u8>,

    /// Pellet-present bitmap by `Map::tile_index`.
    pellets: Vec<bool>,
    pellets_eaten: u32,

    sequence: Sequence,
    /// The next READY! is the long first-start one (two-phase, supplements §4).
    first_start: bool,

    /// Scatter/chase schedule position (dossier §3.2).
    phase_idx: usize,
    phase_ticks: u32,
    mode: Mode,
    /// Frightened ticks remaining; 0 = not frightened.
    fright_ticks_left: u32,
    /// Ghosts eaten on the current energizer (dossier §5.1).
    ghost_chain: u8,

    pac: Pac,
    ghosts: [Ghost; 4],

    /// Post-death global dot counter (dossier §3.10).
    global_counter_active: bool,
    global_counter: u32,
    no_dot_ticks: u32,
    /// Cruise Elroy suspended after a death until Clyde exits (dossier §3.11).
    elroy_suspended: bool,

    /// Fruit ticks remaining on screen; 0 = no fruit.
    fruit_ticks: u32,
    fruit_fired: [bool; 2],

    /// Score popups: (tile, value, ticks remaining).
    popups: Vec<(TilePos, u32, u32)>,
    /// Global sprite-animation counter (ghost legs).
    anim_tick: u32,
    /// Energizer blink counter.
    blink_tick: u32,

    /// Per-tick event buffer, drained by `tick`.
    events: Vec<Event>,
}

fn ghost_index(g: GhostId) -> usize {
    GhostId::ALL.iter().position(|&x| x == g).expect("in ALL")
}

fn symbol_index(s: BonusSymbol) -> u8 {
    match s {
        BonusSymbol::Cherries => 0,
        BonusSymbol::Strawberry => 1,
        BonusSymbol::Peach => 2,
        BonusSymbol::Apple => 3,
        BonusSymbol::Grapes => 4,
        BonusSymbol::Galaxian => 5,
        BonusSymbol::Bell => 6,
        BonusSymbol::Key => 7,
    }
}

impl Game {
    /// A fresh game on `map` at level 1 with `timings::STARTING_LIVES`.
    pub fn new(map: Map, rules: Rules, difficulty: Difficulty, seed: u64) -> Game {
        let dead = Pac {
            pos: PxPos::default(),
            dir: Dir::Left,
            accum: 0,
            pause: 0,
            corner_left: 0,
            corner_step: (0, 0),
            anim_px: 0,
        };
        let ghost = Ghost {
            state: GhostState::InHouse,
            pos: PxPos::default(),
            dir: Dir::Left,
            next_dir: Dir::Left,
            future_dir: Dir::Left,
            accum: 0,
            frightened: false,
            reverse_pending: false,
            exit_right: false,
            dot_counter: 0,
        };
        let mut g = Game {
            rng: Rng::new(seed),
            map,
            rules,
            difficulty,
            seed,
            level: 1,
            score: 0,
            high_score: 0,
            lives: timings::STARTING_LIVES,
            extra_life_awarded: false,
            game_over: false,
            fruit_history: Vec::new(),
            pellets: Vec::new(),
            pellets_eaten: 0,
            sequence: Sequence::Ready { tick: 0 },
            first_start: true,
            phase_idx: 0,
            phase_ticks: 0,
            mode: Mode::Scatter,
            fright_ticks_left: 0,
            ghost_chain: 0,
            pac: dead,
            ghosts: [ghost.clone(), ghost.clone(), ghost.clone(), ghost],
            global_counter_active: false,
            global_counter: 0,
            no_dot_ticks: 0,
            elroy_suspended: false,
            fruit_ticks: 0,
            fruit_fired: [false; 2],
            popups: Vec::new(),
            anim_tick: 0,
            blink_tick: 0,
            events: Vec::new(),
        };
        g.start_level();
        g
    }

    /// Advance exactly one tick. Returns events emitted this tick, in order.
    pub fn tick(&mut self, input: InputFrame) -> Vec<Event> {
        if self.game_over {
            return Vec::new();
        }
        if !matches!(self.sequence, Sequence::Playing) {
            self.sequence_tick();
            return std::mem::take(&mut self.events);
        }
        self.advance_mode_timers(); // phase 1
        self.move_pac(input); // phase 2
        self.eat_pellet(); // phase 3
        self.update_fruit(); // phase 4
        self.evaluate_releases(); // phase 5
        self.move_ghosts(); // phase 6
        self.tick_no_dot_timer(); // phase 7
        self.check_collisions(); // phase 8
        self.check_level_clear(); // phase 9
        self.housekeeping(); // phase 10
        std::mem::take(&mut self.events)
    }

    // --- getters (stub contract) ---------------------------------------

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }
    pub fn score(&self) -> u32 {
        self.score
    }
    pub fn level(&self) -> u32 {
        self.level
    }
    pub fn lives(&self) -> u8 {
        self.lives
    }
    pub fn map(&self) -> &Map {
        &self.map
    }

    // --- extra observability getters (for tests/app; read-only) ---------

    /// Current sequence (tests wait for `Sequence::Playing` without building
    /// a full `RenderState`).
    pub fn sequence(&self) -> Sequence {
        self.sequence
    }
    /// Current global pursuit mode (dossier §3.2).
    pub fn mode(&self) -> Mode {
        self.mode
    }
    /// Pellets (dots + energizers) still on the board.
    pub fn pellets_remaining(&self) -> u32 {
        self.map.pellets_total() - self.pellets_eaten
    }
    /// The tile a ghost's decisions currently steer toward: scatter corner /
    /// chase target / eyes home tile. `None` while the ghost has no target
    /// (frightened random walk, or housed). Exposes dossier §3.5/§3.6
    /// targeting for verification.
    pub fn ghost_target(&self, ghost: GhostId) -> Option<TilePos> {
        let i = ghost_index(ghost);
        match self.ghosts[i].state {
            GhostState::Eyes => Some(self.map.eyes_target()),
            GhostState::Active if !self.ghosts[i].frightened => Some(self.target_tile(i)),
            _ => None,
        }
    }

    // --- render snapshot -------------------------------------------------

    /// Renderer snapshot for the current state. Fills every field.
    pub fn render_state(&self) -> RenderState {
        let first_ready_hidden = matches!(self.sequence, Sequence::Ready { tick }
            if self.first_start && tick < timings::READY_FIRST_NO_ACTORS_TICKS);
        let ghosts_hidden = first_ready_hidden
            || match self.sequence {
                // Ghosts vanish when the death animation proper starts
                // (supplements §3) and at the first level-clear flash
                // (supplements §5).
                Sequence::DeathAnim { .. } => true,
                Sequence::LevelFlash { tick } => tick >= timings::LEVEL_CLEAR_FREEZE_TICKS,
                _ => false,
            };
        // Pac-Man hidden only during the first-start "no actors" READY phase
        // and while a GhostScoreFreeze popup replaces him (supplements §4/§1).
        let pac_visible =
            !first_ready_hidden && !matches!(self.sequence, Sequence::GhostScoreFreeze { .. });
        let ghost_anim = ((self.anim_tick / timings::GHOST_ANIM_HALF_TICKS) % 2) as u8;
        let ghosts = [0usize, 1, 2, 3].map(|i| {
            let g = &self.ghosts[i];
            GhostRender {
                id: GhostId::ALL[i],
                pos: g.pos,
                dir: g.dir,
                state: g.state,
                frightened: g.frightened,
                anim: ghost_anim,
                visible: !ghosts_hidden,
            }
        });
        let energizer_blink_on = match self.sequence {
            // Blink continues through the death freeze/animation and the
            // ghost-eaten pause; steady during READY!/level-clear
            // (supplements §3/§1/§12; READY/clear approximation, timings.rs).
            Sequence::Ready { .. } | Sequence::LevelFlash { .. } => true,
            _ => (self.blink_tick / timings::ENERGIZER_BLINK_HALF_TICKS).is_multiple_of(2),
        };
        RenderState {
            pac_pos: self.pac.pos,
            pac_dir: self.pac.dir,
            pac_anim: ((self.pac.anim_px / timings::PAC_ANIM_PX_PER_FRAME) % 4) as u8,
            ghosts,
            fright_flash: if self.fright_ticks_left > 0 {
                Some(self.fright_is_white())
            } else {
                None
            },
            fruit: if self.fruit_ticks > 0 {
                Some(self.map.fruit_pos().tile())
            } else {
                None
            },
            popups: self.popups.clone(),
            score: self.score,
            high_score: self.high_score,
            lives: self.lives,
            level: self.level,
            fruit_history: self.fruit_history.clone(),
            sequence: self.sequence,
            pellets: self.pellets.clone(),
            energizer_blink_on,
            pac_visible,
        }
    }

    // --- level / spec helpers -------------------------------------------

    fn spec(&self) -> &crate::rules::LevelSpec {
        self.rules.level_spec(self.level, self.difficulty)
    }

    /// Bonus (fruit) fields: on Hard the arcade shows/awards the *board*
    /// symbol, i.e. the Normal row (tables.md A.2 prose; review finding F2).
    fn bonus_spec(&self) -> &crate::rules::LevelSpec {
        self.rules.level_spec(self.level, Difficulty::Normal)
    }

    fn scaled(&self, classic_value: u32) -> u32 {
        self.map.scale_dot_threshold(classic_value)
    }

    /// Cruise Elroy stage 0/1/2 for Blinky (dossier §3.11), honoring the
    /// post-death suspension and custom-map threshold scaling.
    fn elroy_stage(&self) -> u32 {
        if self.elroy_suspended {
            return 0;
        }
        let remaining = self.map.pellets_total() - self.pellets_eaten;
        let spec = self.spec();
        if remaining <= self.scaled(spec.elroy2_dots_left) {
            2
        } else if remaining <= self.scaled(spec.elroy1_dots_left) {
            1
        } else {
            0
        }
    }

    /// Frightened flash phase: white during the even half-periods of the
    /// final `flashes * 2 * half` ticks. `half` shrinks below
    /// `FRIGHT_FLASH_HALF_TICKS` when the level's frightened time is too
    /// short to fit the table's flash count at the nominal rate, so the
    /// Table A.1 "# of Flashes" is always honored exactly.
    fn fright_is_white(&self) -> bool {
        let spec = self.spec();
        let (Some(total), Some(flashes)) = (spec.fright_ticks, spec.fright_flashes) else {
            return false;
        };
        if flashes == 0 {
            return false;
        }
        let half = timings::FRIGHT_FLASH_HALF_TICKS
            .min(total / (flashes * 2))
            .max(1);
        let window = flashes * 2 * half;
        let r = self.fright_ticks_left;
        r <= window && ((window - r) / half).is_multiple_of(2)
    }

    // --- level / life transitions ----------------------------------------

    /// Full board + state reset for the current `self.level` (game start and
    /// every completed level). Pushes the level's fruit symbol into the HUD
    /// history (Hard shows the board symbol — F2).
    fn start_level(&mut self) {
        let (w, h) = (self.map.width(), self.map.height());
        self.pellets = (0..w * h)
            .map(|i| {
                let t = TilePos::new(i % w, i / w);
                matches!(self.map.cell(t), Cell::Dot | Cell::Energizer)
            })
            .collect();
        self.pellets_eaten = 0;
        self.fruit_ticks = 0;
        self.fruit_fired = [false; 2];
        self.reset_schedule_and_fright();
        for g in &mut self.ghosts {
            g.dot_counter = 0; // reset at level start (dossier §3.10)
        }
        self.global_counter_active = false;
        self.global_counter = 0;
        self.no_dot_ticks = 0;
        self.elroy_suspended = false;
        self.rng = Rng::new(self.seed); // dossier §3.8: reseed per level
        self.popups.clear();
        let sym = symbol_index(self.bonus_spec().bonus_symbol);
        self.fruit_history.push(sym);
        if self.fruit_history.len() > FRUIT_HISTORY_LEN {
            self.fruit_history.remove(0);
        }
        self.reset_actors();
        self.sequence = Sequence::Ready { tick: 0 };
    }

    /// Reset after a lost life: pellets preserved (dossier §4.2), global
    /// counter activated at 0, personal counters kept (disabled, not reset),
    /// Elroy suspended until Clyde exits, RNG reseeded, schedule reset.
    fn respawn_after_death(&mut self) {
        self.reset_schedule_and_fright();
        self.global_counter_active = true;
        self.global_counter = 0;
        self.no_dot_ticks = 0;
        self.elroy_suspended = true;
        self.rng = Rng::new(self.seed); // dossier §3.8/§4.2: reseed on death
        self.fruit_ticks = 0; // board redraw clears the fruit (silent)
        self.popups.clear();
        self.reset_actors();
        // Suspension lifts when Clyde moves to exit (dossier §3.11); if this
        // map spawns Clyde outside the house there is nothing to wait for.
        if self.ghosts[3].state != GhostState::InHouse {
            self.elroy_suspended = false;
        }
        self.sequence = Sequence::Ready { tick: 0 };
    }

    fn reset_schedule_and_fright(&mut self) {
        self.phase_idx = 0;
        self.phase_ticks = 0;
        self.mode = self
            .rules
            .schedule(self.level, self.difficulty)
            .first()
            .map(|p| p.mode)
            .unwrap_or(Mode::Scatter);
        self.fright_ticks_left = 0;
        self.ghost_chain = 0;
    }

    /// Restore all actors to their spawn state (dossier §1.3, §4.2).
    fn reset_actors(&mut self) {
        self.pac = Pac {
            pos: self.map.pac_spawn(),
            dir: self.map.pac_spawn_dir(),
            accum: 0,
            pause: 0,
            corner_left: 0,
            corner_step: (0, 0),
            anim_px: 0,
        };
        for i in 0..4 {
            let id = GhostId::ALL[i];
            let pos = self.map.ghost_spawn(id);
            let dir = self.map.ghost_spawn_dir(id);
            let in_house = self.map.cell(pos.tile()) == Cell::House;
            let g = &mut self.ghosts[i];
            g.pos = pos;
            g.dir = dir;
            g.state = if in_house {
                GhostState::InHouse
            } else {
                GhostState::Active
            };
            g.next_dir = dir;
            g.future_dir = dir;
            g.accum = 0;
            g.frightened = false;
            g.reverse_pending = false;
            g.exit_right = false;
        }
        // Outside ghosts need their first look-ahead decision (dossier §3.4).
        for i in 0..4 {
            if self.ghosts[i].state == GhostState::Active {
                let g = &self.ghosts[i];
                let ahead = self.wrap_tile(step_tile(g.pos.tile(), g.dir));
                self.ghosts[i].future_dir = self.choose_dir(i, ahead, self.ghosts[i].dir);
            }
        }
    }

    fn start_next_level(&mut self) {
        self.level += 1;
        self.events
            .push(Event::NextLevelStarted { level: self.level });
        self.start_level();
    }

    fn finish_death(&mut self) {
        self.lives = self.lives.saturating_sub(1);
        self.events.push(Event::LifeLost {
            lives_left: self.lives,
        });
        if self.lives == 0 {
            self.events.push(Event::GameOver);
            self.game_over = true;
            self.sequence = Sequence::DeathAnim {
                tick: timings::DEATH_ANIM_TICKS,
            };
        } else {
            self.respawn_after_death();
        }
    }

    // --- non-Playing sequences -------------------------------------------

    fn sequence_tick(&mut self) {
        match self.sequence {
            Sequence::Playing => unreachable!("sequence_tick only runs during sequences"),
            Sequence::Ready { tick } => {
                let limit = if self.first_start {
                    timings::READY_FIRST_TICKS
                } else {
                    timings::READY_TICKS
                };
                let t = tick + 1;
                if t >= limit {
                    self.first_start = false;
                    self.sequence = Sequence::Playing;
                } else {
                    self.sequence = Sequence::Ready { tick: t };
                }
            }
            Sequence::DeathFreeze { tick } => {
                // Supplements §3: outside ghosts freeze but housed ghosts
                // keep bouncing; sprite anim, energizer blink and the fruit
                // despawn timer keep running.
                for i in 0..4 {
                    if self.ghosts[i].state == GhostState::InHouse {
                        self.house_bounce(i);
                    }
                }
                self.fruit_countdown();
                self.tick_popups();
                self.anim_tick += 1;
                self.blink_tick += 1;
                let t = tick + 1;
                self.sequence = if t >= timings::DEATH_FREEZE_TICKS {
                    Sequence::DeathAnim { tick: 0 }
                } else {
                    Sequence::DeathFreeze { tick: t }
                };
            }
            Sequence::DeathAnim { tick } => {
                // Ghosts hidden; fruit timer still runs (supplements §3).
                self.fruit_countdown();
                self.tick_popups();
                self.blink_tick += 1;
                let t = tick + 1;
                if t >= timings::DEATH_ANIM_TICKS {
                    self.finish_death();
                } else {
                    self.sequence = Sequence::DeathAnim { tick: t };
                }
            }
            Sequence::GhostScoreFreeze { tick, ghost, score } => {
                // Supplements §1: Pac-Man and live ghosts frozen (in-house
                // bounce and all gameplay timers paused); eyes of previously
                // eaten ghosts keep moving; anim/blink/fruit timers run.
                for i in 0..4 {
                    match self.ghosts[i].state {
                        GhostState::Eyes => self.move_eyes(i),
                        GhostState::Entering => self.house_enter(i),
                        _ => {}
                    }
                }
                self.fruit_countdown();
                self.tick_popups();
                self.anim_tick += 1;
                self.blink_tick += 1;
                let t = tick + 1;
                if t >= timings::GHOST_SCORE_FREEZE_TICKS {
                    // Score sprite becomes the eyes (supplements §1). Dead
                    // ghosts never process reversal flags (supplements §12:
                    // eyes skip the alive-ghost AI), so a reversal queued
                    // while this ghost was still alive is discarded.
                    let i = ghost_index(ghost);
                    self.ghosts[i].state = GhostState::Eyes;
                    self.ghosts[i].frightened = false;
                    self.ghosts[i].reverse_pending = false;
                    self.ghosts[i].accum = 0;
                    self.sequence = Sequence::Playing;
                } else {
                    self.sequence = Sequence::GhostScoreFreeze {
                        tick: t,
                        ghost,
                        score,
                    };
                }
            }
            Sequence::LevelFlash { tick } => {
                // Freeze, then 4 white flashes at 12 ticks/phase, then a
                // short blank (supplements §5). Everything static.
                let total = timings::LEVEL_CLEAR_FREEZE_TICKS
                    + timings::LEVEL_CLEAR_FLASHES * 2 * timings::LEVEL_CLEAR_FLASH_HALF_TICKS
                    + timings::LEVEL_CLEAR_BLANK_TICKS;
                let t = tick + 1;
                if t >= total {
                    self.start_next_level();
                } else {
                    self.sequence = Sequence::LevelFlash { tick: t };
                }
            }
        }
    }

    // --- Playing phases ---------------------------------------------------

    /// Phase 1: frightened countdown / scatter-chase schedule. The schedule
    /// timer is frozen while frightened (dossier §3.2).
    fn advance_mode_timers(&mut self) {
        if self.fright_ticks_left > 0 {
            self.fright_ticks_left -= 1;
            if self.fright_ticks_left == 0 {
                self.events.push(Event::FrightenedEnded);
                self.ghost_chain = 0; // supplements §12: chain resets at fright end
                for g in &mut self.ghosts {
                    // No reversal on fright end (dossier §3.3).
                    g.frightened = false;
                }
            }
            return;
        }
        let schedule = self.rules.schedule(self.level, self.difficulty);
        let phase = schedule[self.phase_idx];
        if let Some(len) = phase.ticks
            && self.phase_ticks >= len
        {
            self.phase_idx += 1;
            self.phase_ticks = 0;
            self.mode = schedule[self.phase_idx].mode;
            self.events.push(Event::ModeChanged { mode: self.mode });
            self.queue_reversals();
        }
        self.phase_ticks += 1;
    }

    /// Queue a forced reversal on every board ghost; housed ghosts note the
    /// mode change and will exit rightward instead (dossier §3.3, §3.9).
    /// Eyes are exempt (supplements §12: dead ghosts skip the alive-ghost AI).
    fn queue_reversals(&mut self) {
        for g in &mut self.ghosts {
            match g.state {
                GhostState::Active => g.reverse_pending = true,
                GhostState::InHouse | GhostState::Leaving => g.exit_right = true,
                GhostState::Eyes | GhostState::Entering => {}
            }
        }
    }

    /// Phase 3: eat the pellet under Pac-Man, with all knock-on effects.
    fn eat_pellet(&mut self) {
        let t = self.pac.pos.tile();
        let idx = self.map.tile_index(t);
        if !self.pellets[idx] {
            return;
        }
        match self.map.cell(t) {
            Cell::Dot => {
                self.pellets[idx] = false;
                self.add_score(DOT_POINTS);
                self.pac.pause = timings::DOT_EAT_PAUSE_TICKS; // dossier §2.4
                self.events.push(Event::DotEaten {
                    tile: t,
                    score: DOT_POINTS,
                });
            }
            Cell::Energizer => {
                self.pellets[idx] = false;
                self.add_score(ENERGIZER_POINTS);
                self.pac.pause = timings::ENERGIZER_EAT_PAUSE_TICKS; // dossier §2.4
                self.events.push(Event::EnergizerEaten {
                    tile: t,
                    score: ENERGIZER_POINTS,
                });
                self.energize();
            }
            _ => return,
        }
        self.pellets_eaten += 1;
        self.on_pellet_counters();
    }

    /// Energizer effects (dossier §3.8): always reverse every ghost; start
    /// frightened time only on levels that have it (fright columns `Some`);
    /// the 200-400-800-1600 chain resets per energizer (dossier §5.1).
    fn energize(&mut self) {
        self.queue_reversals();
        self.ghost_chain = 0;
        if let Some(ft) = self.spec().fright_ticks {
            self.fright_ticks_left = ft;
            self.events.push(Event::FrightenedStarted);
            for g in &mut self.ghosts {
                if matches!(
                    g.state,
                    GhostState::Active | GhostState::InHouse | GhostState::Leaving
                ) {
                    g.frightened = true;
                }
            }
        }
    }

    /// Pellet-driven counters: no-dot timer reset, house dot counters with
    /// the global-counter quirk (dossier §3.10), fruit triggers (dossier
    /// §5.2; scaled on custom maps per docs/map-format.md).
    fn on_pellet_counters(&mut self) {
        self.no_dot_ticks = 0;
        if self.global_counter_active {
            self.global_counter += 1;
            let limits = self.rules.global_dot_limits();
            // Equality releases at 7 (Pinky) and 17 (Inky); the counts are
            // checked only at these exact values, which is what makes the
            // "keep the ghosts in the house" trick work (dossier §3.10).
            for i in [1usize, 2] {
                if self.ghosts[i].state == GhostState::InHouse
                    && self.global_counter == self.scaled(limits[i])
                {
                    self.release_ghost(i);
                }
            }
            // At 32: if Clyde is housed the counter is reset AND deactivated
            // (no release); otherwise it stays active forever (dossier §3.10).
            if self.global_counter == self.scaled(limits[3])
                && self.ghosts[3].state == GhostState::InHouse
            {
                self.global_counter = 0;
                self.global_counter_active = false;
            }
        } else if let Some(i) = self.first_housed() {
            // Only the most-preferred housed ghost's counter is active
            // (dossier §3.10; order via first_housed).
            self.ghosts[i].dot_counter += 1;
        }
        // Fruit triggers (dossier §5.2: 70/170 dots; ROM checks equality —
        // supplements §9). Scaled for custom maps.
        let triggers = self.rules.fruit_trigger_dots();
        for (k, &trig) in triggers.iter().enumerate() {
            if !self.fruit_fired[k] && self.pellets_eaten == self.scaled(trig) {
                self.fruit_fired[k] = true;
                self.spawn_fruit();
            }
        }
    }

    fn spawn_fruit(&mut self) {
        let span = timings::FRUIT_TICKS_MAX - timings::FRUIT_TICKS_MIN + 1;
        // RNG consumption point 2 (see module docs).
        self.fruit_ticks = timings::FRUIT_TICKS_MIN + self.rng.next_u32() % span;
        self.events.push(Event::FruitSpawned {
            tile: self.map.fruit_pos().tile(),
        });
    }

    /// Phase 4: fruit eat check, then despawn countdown.
    fn update_fruit(&mut self) {
        if self.fruit_ticks == 0 {
            return;
        }
        if self.pac.pos.tile() == self.map.fruit_pos().tile() {
            // Hard mode awards the displayed (board) symbol's points — F2.
            let points = self.bonus_spec().bonus_points;
            self.add_score(points);
            self.fruit_ticks = 0;
            self.events.push(Event::FruitEaten { score: points });
            self.popups
                .push((self.map.fruit_pos().tile(), points, timings::POPUP_TICKS));
            return;
        }
        self.fruit_countdown();
    }

    fn fruit_countdown(&mut self) {
        if self.fruit_ticks > 0 {
            self.fruit_ticks -= 1;
            if self.fruit_ticks == 0 {
                self.events.push(Event::FruitExpired);
            }
        }
    }

    /// Most-preferred housed ghost: Blinky, Pinky, Inky, Clyde (dossier
    /// §3.10 preference order; Blinky participates only after a revival and
    /// his limit of 0 releases him immediately).
    fn first_housed(&self) -> Option<usize> {
        (0..4).find(|&i| self.ghosts[i].state == GhostState::InHouse)
    }

    /// Phase 5: personal-counter / limit-zero release of the most-preferred
    /// housed ghost. Global-counter releases at 7/17 happen on increment in
    /// `on_pellet_counters`; here the global path only frees limit-0 ghosts
    /// (a revived Blinky).
    fn evaluate_releases(&mut self) {
        let Some(i) = self.first_housed() else { return };
        let release = if self.global_counter_active {
            self.scaled(self.rules.global_dot_limits()[i]) == 0
        } else {
            let limit = self.scaled(self.rules.house_dot_limits(self.level, self.difficulty)[i]);
            self.ghosts[i].dot_counter >= limit
        };
        if release {
            self.release_ghost(i);
        }
    }

    fn release_ghost(&mut self, i: usize) {
        self.ghosts[i].state = GhostState::Leaving;
        self.events.push(Event::GhostReleased {
            ghost: GhostId::ALL[i],
        });
        // Elroy suspension lifts when Clyde moves toward the door
        // (dossier §3.11).
        if i == 3 {
            self.elroy_suspended = false;
        }
    }

    /// Phase 7: force-release timer — time since the last dot was eaten
    /// (dossier §3.10: 4 s on levels 1–4, 3 s from level 5).
    fn tick_no_dot_timer(&mut self) {
        self.no_dot_ticks += 1;
        if self.no_dot_ticks >= self.rules.no_dot_release_ticks(self.level, self.difficulty) {
            self.no_dot_ticks = 0;
            if let Some(i) = self.first_housed() {
                self.release_ghost(i);
            }
        }
    }

    /// Phase 8: collisions, once per tick after both actors moved (dossier
    /// §4.1: tile test, pass-through preserved). Eating a frightened ghost
    /// additionally succeeds when both axis pixel distances are < 4
    /// (supplements §12). At most one ghost is eaten per tick (the freeze
    /// starts immediately; overlapping ghosts are eaten on later ticks).
    fn check_collisions(&mut self) {
        let pt = self.pac.pos.tile();
        for i in 0..4 {
            let g = &self.ghosts[i];
            if !matches!(
                g.state,
                GhostState::Active | GhostState::InHouse | GhostState::Leaving
            ) {
                continue;
            }
            let same_tile = g.pos.tile() == pt;
            if g.frightened {
                let near = (g.pos.x.raw() - self.pac.pos.x.raw()).abs() < 4 * 256
                    && (g.pos.y.raw() - self.pac.pos.y.raw()).abs() < 4 * 256;
                if same_tile || near {
                    self.eat_ghost(i);
                    return;
                }
            } else if same_tile {
                self.events.push(Event::PacDying);
                self.sequence = Sequence::DeathFreeze { tick: 0 };
                return;
            }
        }
    }

    fn eat_ghost(&mut self, i: usize) {
        self.ghost_chain = self.ghost_chain.saturating_add(1);
        let score = self.rules.ghost_chain_score(self.ghost_chain);
        self.add_score(score);
        let ghost = GhostId::ALL[i];
        self.events.push(Event::GhostEaten {
            ghost,
            score,
            chain: self.ghost_chain,
        });
        // The eaten ghost's slot shows the score for the freeze duration;
        // it becomes Eyes when the freeze ends (supplements §1/§11).
        self.sequence = Sequence::GhostScoreFreeze {
            tick: 0,
            ghost,
            score,
        };
    }

    /// Phase 9: the level ends when every pellet is eaten (dossier §5.4).
    /// Skipped if a collision already started a sequence (death wins).
    fn check_level_clear(&mut self) {
        if matches!(self.sequence, Sequence::Playing)
            && self.pellets_eaten == self.map.pellets_total()
        {
            self.events.push(Event::LevelCleared { level: self.level });
            self.fruit_ticks = 0;
            self.popups.clear();
            self.sequence = Sequence::LevelFlash { tick: 0 };
        }
    }

    /// Phase 10: extra life (once, at the rules threshold — dossier §5.3),
    /// popup expiry, animation/blink counters, high score.
    fn housekeeping(&mut self) {
        if !self.extra_life_awarded && self.score >= self.rules.extra_life_score() {
            self.extra_life_awarded = true;
            self.lives = self.lives.saturating_add(1);
            self.events.push(Event::ExtraLife);
        }
        self.tick_popups();
        self.anim_tick += 1;
        self.blink_tick += 1;
    }

    fn tick_popups(&mut self) {
        for p in &mut self.popups {
            p.2 = p.2.saturating_sub(1);
        }
        self.popups.retain(|p| p.2 > 0);
    }

    fn add_score(&mut self, points: u32) {
        self.score = self.score.saturating_add(points);
        self.high_score = self.high_score.max(self.score);
    }

    fn wrap_tile(&self, t: TilePos) -> TilePos {
        TilePos::new(
            t.x.rem_euclid(self.map.width()),
            t.y.rem_euclid(self.map.height()),
        )
    }

    // --- speeds ------------------------------------------------------------

    /// Pac-Man speed (Fix8/tick) from the MOVEMENT columns; the `~` dots
    /// columns are never used — the eating pauses are modeled explicitly
    /// (dossier §2.4/§8.2, rules::LevelSpec docs).
    fn pac_speed(&self) -> i32 {
        let spec = self.spec();
        let pct = if self.fright_ticks_left > 0 {
            spec.fright_pac_speed_pct.unwrap_or(spec.pac_speed_pct)
        } else {
            spec.pac_speed_pct
        };
        Fix8::speed_from_percent(pct).raw()
    }

    /// Ghost speed (Fix8/tick) by state: eyes 2 px flat (supplements §2),
    /// house/door 40% (supplements §10), tunnel slowdown always enforced for
    /// live ghosts (dossier §2.5), frightened speed, Elroy overrides
    /// (dossier §3.11).
    fn ghost_speed(&self, i: usize) -> i32 {
        let g = &self.ghosts[i];
        let pct = match g.state {
            GhostState::Eyes | GhostState::Entering => timings::EYES_SPEED_PCT,
            GhostState::InHouse | GhostState::Leaving => timings::HOUSE_SPEED_PCT,
            GhostState::Active => {
                let spec = self.spec();
                if self.map.is_tunnel(g.pos.tile()) {
                    spec.ghost_tunnel_speed_pct
                } else if g.frightened {
                    spec.fright_ghost_speed_pct.unwrap_or(spec.ghost_speed_pct)
                } else if i == 0 {
                    match self.elroy_stage() {
                        2 => spec.elroy2_speed_pct,
                        1 => spec.elroy1_speed_pct,
                        _ => spec.ghost_speed_pct,
                    }
                } else {
                    spec.ghost_speed_pct
                }
            }
        };
        Fix8::speed_from_percent(pct).raw()
    }
}

/// One tile step in direction `d` (no wrapping; callers wrap when needed).
fn step_tile(t: TilePos, d: Dir) -> TilePos {
    let (dx, dy) = d.delta();
    TilePos::new(t.x + dx, t.y + dy)
}

/// Deterministic 32-bit xorshift PRNG (documented; replays store the seed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng(pub u32);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        // Fold the 64-bit seed; avoid the all-zero state.
        let s = (seed as u32) ^ ((seed >> 32) as u32) ^ 0x9E37_79B9;
        Rng(if s == 0 { 0x9E37_79B9 } else { s })
    }
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
