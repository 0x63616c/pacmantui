//! Shared core types — the cross-module contract.
//!
//! Coordinate conventions (dossier, docs/research/dossier-mechanics.md):
//! - The classic screen is a 28x36 grid of 8x8-px tiles (224x288 px); maps may
//!   use other dimensions. Tile (0,0) is top-left; x grows right, y grows down.
//! - Actor positions are pixel-precise centers with 1/256-px subpixel accuracy
//!   ([`Fix8`]). An actor occupies the tile containing its center.
//! - One sim tick = one arcade frame at 60.606061 Hz. 100% speed = 1.25 px/tick.

pub const TILE_PX: i32 = 8;
/// 100% speed in Fix8 pixels per tick: 1.25 px * 256 = 320.
pub const FULL_SPEED_FIX8: i32 = 320;
/// Nominal tick rate, ticks per second (arcade 60.606061 Hz).
pub const TICK_HZ: f64 = 60.606_061;

/// Fixed-point pixels, 1/256 px resolution. Wrapper over raw i32 subpixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Fix8(pub i32);

impl Fix8 {
    pub const fn from_px(px: i32) -> Self {
        Fix8(px * 256)
    }
    pub const fn px(self) -> i32 {
        self.0.div_euclid(256)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
    /// Percent of full speed → Fix8 advance per tick (rounded to nearest).
    pub const fn speed_from_percent(percent: u32) -> Self {
        Fix8(((FULL_SPEED_FIX8 as i64 * percent as i64 + 50) / 100) as i32)
    }
}

/// Direction; enum order IS the arcade tie-break priority (Up > Left > Down > Right).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    Left,
    Down,
    Right,
}

impl Dir {
    pub const IN_PRIORITY_ORDER: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];
    pub const fn opposite(self) -> Dir {
        match self {
            Dir::Up => Dir::Down,
            Dir::Left => Dir::Right,
            Dir::Down => Dir::Up,
            Dir::Right => Dir::Left,
        }
    }
    /// Unit tile delta (dx, dy).
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Left => (-1, 0),
            Dir::Down => (0, 1),
            Dir::Right => (1, 0),
        }
    }
}

/// Tile coordinates. May lie outside the map (scatter targets, tunnel exits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub const fn new(x: i32, y: i32) -> Self {
        TilePos { x, y }
    }
    /// Squared euclidean distance (arcade ghost targeting metric).
    pub const fn dist_sq(self, o: TilePos) -> i64 {
        let (dx, dy) = ((self.x - o.x) as i64, (self.y - o.y) as i64);
        dx * dx + dy * dy
    }
}

/// Pixel-precise position (of an actor center).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PxPos {
    pub x: Fix8,
    pub y: Fix8,
}

impl PxPos {
    pub const fn tile(self) -> TilePos {
        TilePos {
            x: self.x.px().div_euclid(TILE_PX),
            y: self.y.px().div_euclid(TILE_PX),
        }
    }
    /// Center of a tile, in pixels.
    pub const fn tile_center(t: TilePos) -> PxPos {
        PxPos {
            x: Fix8::from_px(t.x * TILE_PX + TILE_PX / 2),
            y: Fix8::from_px(t.y * TILE_PX + TILE_PX / 2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GhostId {
    Blinky,
    Pinky,
    Inky,
    Clyde,
}

impl GhostId {
    pub const ALL: [GhostId; 4] = [
        GhostId::Blinky,
        GhostId::Pinky,
        GhostId::Inky,
        GhostId::Clyde,
    ];
}

/// Global pursuit mode (scatter/chase alternation per schedule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Scatter,
    Chase,
}

/// Per-ghost activity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostState {
    /// Bouncing inside the house (not yet released).
    InHouse,
    /// Moving from house interior through the door to the board.
    Leaving,
    /// On the board following mode/frightened rules.
    Active,
    /// Eaten: eyes returning to the house door.
    Eyes,
    /// Eyes descending into the house before revival.
    Entering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Normal,
    Hard,
}

/// Player input sampled once per tick. `dir` is the currently-held/buffered
/// desired direction (app translates key events into this).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputFrame {
    pub dir: Option<Dir>,
}

/// Simulation events emitted by `Game::tick` (for app/render/sound/tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    DotEaten {
        tile: TilePos,
        score: u32,
    },
    EnergizerEaten {
        tile: TilePos,
        score: u32,
    },
    FruitSpawned {
        tile: TilePos,
    },
    FruitExpired,
    FruitEaten {
        score: u32,
    },
    GhostEaten {
        ghost: GhostId,
        score: u32,
        chain: u8,
    },
    ModeChanged {
        mode: Mode,
    },
    FrightenedStarted,
    FrightenedEnded,
    GhostReleased {
        ghost: GhostId,
    },
    ExtraLife,
    PacDying,
    LifeLost {
        lives_left: u8,
    },
    LevelCleared {
        level: u32,
    },
    NextLevelStarted {
        level: u32,
    },
    GameOver,
}

/// What the death/level-clear/global freeze animation phase is, for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sequence {
    #[default]
    Playing,
    /// Frozen just after losing a life, before the death animation.
    DeathFreeze { tick: u32 },
    /// Pac-Man death animation frame counter.
    DeathAnim { tick: u32 },
    /// Board flash after clearing a level (flash phase derivable from tick).
    LevelFlash { tick: u32 },
    /// Brief freeze showing ghost score after eating a ghost.
    GhostScoreFreeze {
        tick: u32,
        ghost: GhostId,
        score: u32,
    },
    /// READY! countdown before control starts.
    Ready { tick: u32 },
}

/// Snapshot of everything the renderer needs. Produced by `sim`, consumed by
/// `render`; must stay free of sim internals.
#[derive(Debug, Clone)]
pub struct RenderState {
    pub pac_pos: PxPos,
    pub pac_dir: Dir,
    /// 0..=N animation phase for the chomp cycle; render maps to sprite frames.
    pub pac_anim: u8,
    pub ghosts: [GhostRender; 4],
    /// Frightened flash: Some(true)=white phase, Some(false)=blue, None=not frightened.
    pub fright_flash: Option<bool>,
    pub fruit: Option<TilePos>,
    /// Score popups to draw ((tile, value, ticks_remaining)).
    pub popups: Vec<(TilePos, u32, u32)>,
    pub score: u32,
    pub high_score: u32,
    pub lives: u8,
    pub level: u32,
    /// Fruit symbols to show in the HUD strip (most recent levels).
    pub fruit_history: Vec<u8>,
    pub sequence: Sequence,
    /// Bit per map tile: pellet still present (indexing via map dims).
    pub pellets: Vec<bool>,
    pub energizer_blink_on: bool,
    /// Whether Pac-Man is drawn at all. The sim clears this during the
    /// first-start READY "no actors" phase and while a GhostScoreFreeze
    /// score popup replaces him; render draws nothing when false.
    pub pac_visible: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct GhostRender {
    pub id: GhostId,
    pub pos: PxPos,
    pub dir: Dir,
    pub state: GhostState,
    /// True while this ghost is frightened (affects sprite).
    pub frightened: bool,
    pub anim: u8,
    /// Hidden ghosts (during some sequences) aren't drawn.
    pub visible: bool,
}
