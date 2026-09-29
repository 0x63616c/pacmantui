//! Timing/config constants the dossier does not specify numerically.
//!
//! Every constant carries provenance: DOSSIER (exact, docs/research/
//! dossier-mechanics.md), or SUPPLEMENT (ROM-derived, docs/research/
//! arcade-supplements.md, section given; confidence as stated there), or
//! APPROXIMATION (neither source pins the value; the comment says why the
//! chosen value is reasonable). Where the ROM value is a phase window
//! (supplements §0.1/§13), the recommended phase-locked value is used.
//! Units: ticks at 60.606061 Hz unless stated.

/// Starting lives. SUPPLEMENT §6: MAME factory DIP default = 3 lives,
/// ROM decode confirmed at `#26D0` (options 1/2/3/5).
pub const STARTING_LIVES: u8 = 3;

/// Extra life score threshold. SUPPLEMENT §6: bonus-life DIP default 10,000
/// (ROM bonus table at `0x2728` = 10/15/20/none in BCD thousands).
/// `rules::Rules::extra_life_score()` carries the same value; the sim awards
/// at the rules threshold.
pub const EXTRA_LIFE_SCORE: u32 = 10_000;

/// READY! total duration at first game start (two phases, intro jingle).
/// SUPPLEMENT §4: phase A ("PLAYER ONE" + "READY!", no actors) 133–138 f +
/// phase B (actors visible, "READY!" only) 115–120 f; phase-locked values
/// 138 + 120 = 258 (§13).
pub const READY_FIRST_TICKS: u32 = 258;

/// First-start phase A length: "PLAYER ONE" shown, NO actors on screen.
/// SUPPLEMENT §4/§12: timed task `57 01 00` = 133–138 f, phase-locked 138.
pub const READY_FIRST_NO_ACTORS_TICKS: u32 = 138;

/// READY! duration on subsequent lives and every new level (actors visible
/// throughout). SUPPLEMENT §4: state 9 holds `54 00 00` + `54 06 00` =
/// 115–120 f, phase-locked 120 (§13).
pub const READY_TICKS: u32 = 120;

/// Freeze between fatal collision and the ghosts disappearing: everything
/// stops in place, all actors visible (housed ghosts keep bouncing, sprite
/// animation and energizer blink continue). SUPPLEMENT §3: counter `4DC5`
/// threshold 0x78 half-frames = exactly 60 frames.
pub const DEATH_FREEZE_TICKS: u32 = 60;

/// Ghosts-hidden death phase: pre-death sprite 30 f + animation proper
/// 82.5 f + final "pop"/blank 47.5 f = 160 f (SUPPLEMENT §3 thresholds
/// 0x78→0x1B8 half-frames: collision → end of death state = 220 f, minus the
/// 60 f freeze). The renderer maps this span onto its 8 animation frames.
pub const DEATH_ANIM_TICKS: u32 = 160;

/// Global freeze while an eaten ghost's 200/400/800/1600 score is shown:
/// Pac-Man hidden, live ghosts and all gameplay timers frozen; eyes of
/// previously eaten ghosts keep moving; sprite animation, energizer blink
/// and the fruit despawn timer keep running. SUPPLEMENT §1: timed task
/// `4A 03 00` = 55–60 f, phase-locked 60 (§13).
pub const GHOST_SCORE_FREEZE_TICKS: u32 = 60;

/// Eyes (eaten ghost) speed, percent of full speed. SUPPLEMENT §2: exactly
/// 2 px/frame unconditionally (tunnel included) = 160% of 1.25 px/frame.
/// 160% of `FULL_SPEED_FIX8` (320) = 512 = exactly 2 px, no rounding.
pub const EYES_SPEED_PCT: u32 = 160;

/// Ghost speed inside the house (bounce), door transit outward, and the
/// post-revival re-exit, percent. SUPPLEMENT §10: `4D94` mask 0x55 = 1 px
/// every other frame = 0.5 px/frame = 40%.
pub const HOUSE_SPEED_PCT: u32 = 40;

/// In-house bounce amplitude around the slot center, pixels. SUPPLEMENT §10:
/// ghosts oscillate ±4 px (arcade Y=0x78..0x80 around slot Y=0x7C).
pub const HOUSE_BOUNCE_PX: i32 = 4;

/// Level-clear: freeze before flashing (Pac-Man + ghosts static, sprite
/// animation and energizer blink stop). SUPPLEMENT §5: `54 00 00` =
/// 115–121 f, phase-locked 120 (§13).
pub const LEVEL_CLEAR_FREEZE_TICKS: u32 = 120;

/// Level-clear: number of maze white flashes. SUPPLEMENT §5: 8 alternating
/// color states = 4 white flashes (exact ROM count). Ghosts are erased at
/// the first flash; Pac-Man stays on screen.
pub const LEVEL_CLEAR_FLASHES: u32 = 4;

/// Level-clear: ticks per color phase (white or blue). SUPPLEMENT §5: each
/// state holds `42 00 00`, phase-locked 12 f (white↔blue period 24 f).
pub const LEVEL_CLEAR_FLASH_HALF_TICKS: u32 = 12;

/// Level-clear: blank screen after the flashes before the next level's
/// READY!. SUPPLEMENT §5: `43 00 00` = 13–18 f, phase-locked 18 (§13).
pub const LEVEL_CLEAR_BLANK_TICKS: u32 = 18;

/// Fruit visible duration [min, max] ticks, pick uniform in range via the
/// sim RNG. SUPPLEMENT §9: despawn task `8A 04 00` = 10 ticks of the global
/// 1-second (60-frame) clock ⇒ 541–600 frames depending on spawn phase
/// ("always between nine and ten seconds", dossier §5.2). DOCUMENTED
/// DEVIATION: the ROM's spread comes from clock-phase aliasing, not a PRNG;
/// a uniform draw over the same window is distribution-equivalent
/// (supplements §9 "Implementation") but consumes the sim RNG.
pub const FRUIT_TICKS_MIN: u32 = 541;
pub const FRUIT_TICKS_MAX: u32 = 600;

/// Fruit score popup display duration. SUPPLEMENT §11: cleared by its own
/// timed task, "~2 s"; timer byte not individually verified in the original
/// ROM — documented APPROXIMATION (recommended 2.0 s = 120 f).
pub const POPUP_TICKS: u32 = 120;

/// Pac-Man freeze after eating a dot, ticks. DOSSIER §2.4 (Ch.2 "Speed":
/// "stops moving for one frame").
pub const DOT_EAT_PAUSE_TICKS: u32 = 1;

/// Pac-Man freeze after eating an energizer, ticks. DOSSIER §2.4 (3 frames).
pub const ENERGIZER_EAT_PAUSE_TICKS: u32 = 3;

/// Frightened flash: ticks per color phase (white or blue) at the end of
/// frightened time; the flash window is `flashes * 2 * this` ticks.
/// APPROXIMATION: the dossier gives only the flash COUNT (Table A.1) and the
/// supplements did not extract the ROM flash rate (`#0AC3`); 14 f/phase is
/// the widely used emulation value (~2.2 Hz) and reproduces the documented
/// counts exactly.
pub const FRIGHT_FLASH_HALF_TICKS: u32 = 14;

/// Energizer blink half-period (on/off), ticks. APPROXIMATION: blink rate
/// (`#0C0D`) not extracted by the supplements; 10 f ≈ the arcade's ~3 Hz
/// blink. Blinking continues during the death freeze and ghost-eaten pause
/// and stops during the level-clear freeze (supplements §3/§1/§12).
pub const ENERGIZER_BLINK_HALF_TICKS: u32 = 10;

/// Ghost leg-animation half-period, ticks. SUPPLEMENT §12: 8-frame sprite
/// cycle (`4DC0`) over the 2 leg frames = 4 f each; keeps running during the
/// ghost-eaten pause and the death freeze, stops during level-clear.
pub const GHOST_ANIM_HALF_TICKS: u32 = 4;

/// Pac-Man chomp animation: pixels moved per animation phase step (the
/// 4-phase cycle advances only while actually moving). APPROXIMATION: 2 px
/// per phase ⇒ a full chomp every 8 px, the arcade's visual rate at level-1
/// speed (1 px/f).
pub const PAC_ANIM_PX_PER_FRAME: u32 = 2;
