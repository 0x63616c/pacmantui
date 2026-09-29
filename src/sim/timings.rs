//! Timing/config constants the dossier does not specify numerically.
//!
//! Every constant carries provenance: DOSSIER (exact), SUPPLEMENT (from
//! docs/research/arcade-supplements.md with its confidence level), or
//! PROVISIONAL (placeholder pending that research; must not ship unreviewed).
//! Units: ticks at 60.606061 Hz unless stated.

/// Starting lives. PROVISIONAL (arcade DIP default 3; confirm via supplements).
pub const STARTING_LIVES: u8 = 3;

/// Extra life score threshold. PROVISIONAL (arcade DIP default 10_000).
pub const EXTRA_LIFE_SCORE: u32 = 10_000;

/// READY! duration at game start (with intro music). PROVISIONAL.
pub const READY_FIRST_TICKS: u32 = 254;

/// READY! duration on subsequent lives/levels. PROVISIONAL.
pub const READY_TICKS: u32 = 120;

/// Freeze between fatal collision and death animation start. PROVISIONAL.
pub const DEATH_FREEZE_TICKS: u32 = 60;

/// Pac-Man death animation length. PROVISIONAL.
pub const DEATH_ANIM_TICKS: u32 = 120;

/// Global freeze while a ghost's score is shown after being eaten. PROVISIONAL.
pub const GHOST_SCORE_FREEZE_TICKS: u32 = 60;

/// Eyes return speed as percent of full speed. PROVISIONAL (qualitative
/// "faster than normal" in most sources).
pub const EYES_SPEED_PCT: u32 = 150;

/// Ghost speed inside the house / crossing the door, percent. PROVISIONAL.
pub const HOUSE_SPEED_PCT: u32 = 40;

/// Level-clear: freeze before flashing starts. PROVISIONAL.
pub const LEVEL_CLEAR_FREEZE_TICKS: u32 = 120;

/// Level-clear: number of maze white flashes. PROVISIONAL.
pub const LEVEL_CLEAR_FLASHES: u32 = 4;

/// Level-clear: ticks per half-flash (blue↔white). PROVISIONAL.
pub const LEVEL_CLEAR_FLASH_HALF_TICKS: u32 = 14;

/// Fruit visible duration range [min, max] ticks; actual pick is seeded-random
/// in range. DOSSIER: "9-10 seconds" (frame mechanism via supplements).
pub const FRUIT_TICKS_MIN: u32 = 546;
pub const FRUIT_TICKS_MAX: u32 = 606;

/// Score popup (fruit) display duration. PROVISIONAL.
pub const POPUP_TICKS: u32 = 120;

/// Pac-Man freeze after eating a dot, ticks. DOSSIER (Ch.2 Speed: 1 frame).
pub const DOT_EAT_PAUSE_TICKS: u32 = 1;

/// Pac-Man freeze after eating an energizer, ticks. DOSSIER (3 frames).
pub const ENERGIZER_EAT_PAUSE_TICKS: u32 = 3;
