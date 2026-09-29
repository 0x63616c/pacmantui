//! Sequence phase policy — the single home for what each [`Sequence`]
//! phase *means* while play is frozen.
//!
//! For every phase, ONE declaration ([`policy`]) covers:
//!
//! - **(a)** which per-tick subsystems keep running ([`Subsystems`]),
//! - **(b)** actor visibility for the render snapshot (pac / ghosts,
//!   including the tick-dependent cases), and
//! - **(c)** how long the phase lasts and what ends it ([`PhaseEnd`]).
//!
//! `Game::sequence_tick` and `Game::render_state` are interpreters of this
//! table: they hand-list nothing per phase themselves. The arcade citations
//! (supplements §N = docs/research/arcade-supplements.md, dossier §N =
//! docs/research/dossier-mechanics.md) live on the rows they justify.
//!
//! Why one home: both post-validation sim bugs were a phase row transcribed
//! in one interpreter but not another — F2 (fa29a31): the DeathFreeze arm
//! forgot `house_leave` for Leaving ghosts; fb11084: `render_state` forgot
//! to hide the actors at game over. A row declared here reaches every
//! interpreter at once, and the boundary tests (`tests/sim_sequences.rs`,
//! `tests/sim_house.rs`, `tests/sim_modes.rs`) verify the table itself.

use super::timings;
use crate::types::{GhostId, Sequence};

/// (a) Which frozen-play subsystems still run each tick of a phase. One
/// bool per subsystem the sequence arms distinguish; everything not listed
/// (schedule/fright timers, pac control, ghost decisions, collisions,
/// releases) is frozen in every non-Playing phase.
#[derive(Debug, Clone, Copy)]
pub(super) struct Subsystems {
    /// Housed ghosts keep bouncing (`house_bounce`).
    pub house_bounce: bool,
    /// Leaving ghosts finish their door transit (`house_leave`).
    pub house_leave: bool,
    /// Eyes keep flying home and descending into the house
    /// (`move_eyes` / `house_enter`).
    pub eyes: bool,
    /// The fruit despawn timer keeps counting (`fruit_countdown`).
    pub fruit: bool,
    /// Score popups keep expiring (`tick_popups`).
    pub popups: bool,
    /// The ghost leg-animation counter advances (`anim_tick`).
    pub sprite_anim: bool,
    /// The energizer blink counter advances (`blink_tick`).
    pub energizer_blink: bool,
}

impl Subsystems {
    /// Everything frozen. The READY! and level-flash rows, and the base for
    /// the struct-update syntax in the other rows.
    pub(super) const NONE: Subsystems = Subsystems {
        house_bounce: false,
        house_leave: false,
        eyes: false,
        fruit: false,
        popups: false,
        sprite_anim: false,
        energizer_blink: false,
    };
}

/// (c) What happens when a timed phase's duration elapses. The mutations
/// live in `Game::sequence_tick`'s interpreter match; *which* end a phase
/// gets is declared here, per row.
#[derive(Debug, Clone, Copy)]
pub(super) enum PhaseEnd {
    /// READY! over: control starts (and a first start is consumed).
    ControlStarts,
    /// Death freeze over: the ghosts vanish and the death animation proper
    /// starts (supplements §3).
    GhostsVanish,
    /// Death animation over: the life is lost — respawn or game over.
    DeathResolves,
    /// Ghost-score pause over: the score sprite becomes the eaten ghost's
    /// eyes and play resumes (supplements §1). Dead ghosts never process
    /// reversal flags (supplements §12: eyes skip the alive-ghost AI), so a
    /// reversal queued while the ghost was alive is discarded.
    EyesReplaceScore(GhostId),
    /// Flash + blank over: the next level begins.
    NextLevelStarts,
}

/// One phase's complete policy: (a) subsystems, (b) visibility, (c) timing.
#[derive(Debug, Clone, Copy)]
pub(super) struct PhasePolicy {
    /// (a) Subsystems that keep running while play is frozen. Never
    /// interpreted for `Playing` (the full pipeline runs in `Game::tick`).
    pub run: Subsystems,
    /// (b) Whether Pac-Man is drawn.
    pub pac_visible: bool,
    /// (b) Whether the ghosts are drawn.
    pub ghosts_visible: bool,
    /// (b) Energizers steady-on instead of blinking from the counter.
    pub energizer_steady: bool,
    /// (c) `(duration, end)` for the timed phases; `None` for `Playing`.
    pub timing: Option<(u32, PhaseEnd)>,
}

/// THE table: the policy for `seq`, given the two bits of game context the
/// rows depend on (`first_start`: the next READY! is the long two-phase
/// one; `game_over`: the sequence is frozen in place forever).
pub(super) fn policy(seq: Sequence, first_start: bool, game_over: bool) -> PhasePolicy {
    let mut p = match seq {
        // Normal play: everything visible and blinking; the per-tick
        // pipeline in `Game::tick` runs instead of `run`.
        Sequence::Playing => PhasePolicy {
            run: Subsystems::NONE,
            pac_visible: true,
            ghosts_visible: true,
            energizer_steady: false,
            timing: None,
        },
        // READY! countdown. Supplements §4: the first start is two-phase —
        // phase A ("PLAYER ONE" + "READY!") shows NO actors for
        // READY_FIRST_NO_ACTORS_TICKS, phase B shows them; every later
        // READY! shows the actors throughout (phase-locked durations, §13).
        // Everything is static; energizers steady on (READY/clear
        // approximation, timings.rs).
        Sequence::Ready { tick } => {
            let actors_on = !(first_start && tick < timings::READY_FIRST_NO_ACTORS_TICKS);
            let duration = if first_start {
                timings::READY_FIRST_TICKS
            } else {
                timings::READY_TICKS
            };
            PhasePolicy {
                run: Subsystems::NONE,
                pac_visible: actors_on,
                ghosts_visible: actors_on,
                energizer_steady: true,
                timing: Some((duration, PhaseEnd::ControlStarts)),
            }
        }
        // Post-collision freeze, all actors visible in place. Supplements
        // §3: outside ghosts freeze but housed ghosts keep bouncing; sprite
        // anim, energizer blink and the fruit despawn timer keep running.
        // Supplements §10: house movement incl. outward door transit
        // continues during the death freeze (only the ghost-eaten pause
        // halts it), so Leaving ghosts finish their exit; once Active they
        // freeze like the other outside ghosts (the F2 row).
        Sequence::DeathFreeze { .. } => PhasePolicy {
            run: Subsystems {
                house_bounce: true,
                house_leave: true,
                fruit: true,
                popups: true,
                sprite_anim: true,
                energizer_blink: true,
                ..Subsystems::NONE
            },
            pac_visible: true,
            ghosts_visible: true,
            energizer_steady: false,
            timing: Some((timings::DEATH_FREEZE_TICKS, PhaseEnd::GhostsVanish)),
        },
        // Death animation proper: ghosts hidden (supplements §3); the fruit
        // timer, popups and the energizer blink still run.
        Sequence::DeathAnim { .. } => PhasePolicy {
            run: Subsystems {
                fruit: true,
                popups: true,
                energizer_blink: true,
                ..Subsystems::NONE
            },
            pac_visible: true,
            ghosts_visible: false,
            energizer_steady: false,
            timing: Some((timings::DEATH_ANIM_TICKS, PhaseEnd::DeathResolves)),
        },
        // Ghost-eaten pause: the 200/400/800/1600 score sprite replaces
        // Pac-Man. Supplements §1: Pac-Man and live ghosts frozen (in-house
        // bounce and all gameplay timers paused); eyes of previously eaten
        // ghosts keep moving; anim/blink/fruit timers run.
        Sequence::GhostScoreFreeze { ghost, .. } => PhasePolicy {
            run: Subsystems {
                eyes: true,
                fruit: true,
                popups: true,
                sprite_anim: true,
                energizer_blink: true,
                ..Subsystems::NONE
            },
            pac_visible: false,
            ghosts_visible: true,
            energizer_steady: false,
            timing: Some((
                timings::GHOST_SCORE_FREEZE_TICKS,
                PhaseEnd::EyesReplaceScore(ghost),
            )),
        },
        // Level clear: freeze, then 4 white flashes at 12 ticks/phase, then
        // a short blank (supplements §5, phase-locked §13). Everything
        // static; energizers steady on; the ghosts are erased at the first
        // flash (LEVEL_CLEAR_FREEZE_TICKS) while Pac-Man stays on screen.
        Sequence::LevelFlash { tick } => PhasePolicy {
            run: Subsystems::NONE,
            pac_visible: true,
            ghosts_visible: tick < timings::LEVEL_CLEAR_FREEZE_TICKS,
            energizer_steady: true,
            timing: Some((
                timings::LEVEL_CLEAR_FREEZE_TICKS
                    + timings::LEVEL_CLEAR_FLASHES * 2 * timings::LEVEL_CLEAR_FLASH_HALF_TICKS
                    + timings::LEVEL_CLEAR_BLANK_TICKS,
                PhaseEnd::NextLevelStarts,
            )),
        },
    };
    // Game over hides everything, whatever phase the sequence froze in (it
    // parks on the death animation's last frame, which would otherwise
    // leave the "pop" remnant drawn) — the fb11084 row.
    if game_over {
        p.pac_visible = false;
        p.ghosts_visible = false;
    }
    p
}

/// The phase-local tick of a non-Playing sequence.
pub(super) fn tick_of(seq: Sequence) -> u32 {
    match seq {
        Sequence::Playing => unreachable!("Playing has no phase tick"),
        Sequence::Ready { tick }
        | Sequence::DeathFreeze { tick }
        | Sequence::DeathAnim { tick }
        | Sequence::LevelFlash { tick }
        | Sequence::GhostScoreFreeze { tick, .. } => tick,
    }
}

/// The same phase with its tick replaced (payload fields preserved).
pub(super) fn with_tick(seq: Sequence, tick: u32) -> Sequence {
    match seq {
        Sequence::Playing => unreachable!("Playing has no phase tick"),
        Sequence::Ready { .. } => Sequence::Ready { tick },
        Sequence::DeathFreeze { .. } => Sequence::DeathFreeze { tick },
        Sequence::DeathAnim { .. } => Sequence::DeathAnim { tick },
        Sequence::LevelFlash { .. } => Sequence::LevelFlash { tick },
        Sequence::GhostScoreFreeze { ghost, score, .. } => {
            Sequence::GhostScoreFreeze { tick, ghost, score }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fb11084 class of bug: game over must hide BOTH actors in EVERY
    /// phase (the sequence freezes wherever it was), so the overlay is an
    /// invariant of the whole table, not a row of one interpreter.
    #[test]
    fn game_over_hides_actors_in_every_phase() {
        let phases = [
            Sequence::Playing,
            Sequence::Ready { tick: 0 },
            Sequence::Ready { tick: 200 },
            Sequence::DeathFreeze { tick: 0 },
            Sequence::DeathAnim { tick: 0 },
            Sequence::DeathAnim {
                tick: timings::DEATH_ANIM_TICKS,
            },
            Sequence::LevelFlash { tick: 0 },
            Sequence::LevelFlash { tick: 200 },
            Sequence::GhostScoreFreeze {
                tick: 0,
                ghost: GhostId::Blinky,
                score: 200,
            },
        ];
        for seq in phases {
            for first_start in [false, true] {
                let p = policy(seq, first_start, true);
                assert!(
                    !p.pac_visible && !p.ghosts_visible,
                    "game over must hide actors during {seq:?} (first_start={first_start})"
                );
            }
        }
    }
}
