//! Map format parsing, validation, and geometry queries.
//!
//! Format spec: docs/map-format.md (owned by W1-MAP together with this module).
//! Shipped maps live in `maps/` and are embedded via `include_str!`.
//!
//! Owner: W1-MAP agent. Public signatures are the contract; extend, don't break.

use crate::types::{Dir, GhostId, PxPos, TilePos};

/// Static content of one grid cell (collectible state lives in `sim`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Wall,
    /// Walkable, no collectible.
    Path,
    Dot,
    Energizer,
    /// Ghost-house door: crossable only by Leaving/Entering ghosts.
    Door,
    /// Walkable tunnel tile (ghost slowdown zone; may extend off-grid via warps).
    Tunnel,
    /// Ghost-house interior floor.
    House,
}

#[derive(Debug, Clone)]
pub struct Map {
    // W1-MAP defines internals.
    _private: (),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    Io(String),
    Syntax { line: usize, msg: String },
    /// Semantic validation failure with a human-actionable message.
    Invalid(String),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::Io(e) => write!(f, "map io error: {e}"),
            MapError::Syntax { line, msg } => write!(f, "map syntax error (line {line}): {msg}"),
            MapError::Invalid(msg) => write!(f, "invalid map: {msg}"),
            MapError::UnsupportedVersion(v) => write!(f, "unsupported map format version {v}"),
        }
    }
}

impl std::error::Error for MapError {}

impl Map {
    /// Parse and fully validate a map document.
    pub fn parse(src: &str) -> Result<Map, MapError> {
        let _ = src;
        todo!("W1-MAP")
    }

    /// The embedded classic map (must parse+validate; covered by tests).
    pub fn classic() -> Map {
        todo!("W1-MAP")
    }

    /// The embedded custom map.
    pub fn custom() -> Map {
        todo!("W1-MAP")
    }

    pub fn load_file(path: &std::path::Path) -> Result<Map, MapError> {
        let _ = path;
        todo!("W1-MAP")
    }

    // --- identity ---
    /// Display name (from metadata).
    pub fn name(&self) -> &str {
        todo!("W1-MAP")
    }
    /// Stable id for score separation: metadata id + content hash.
    pub fn id(&self) -> &str {
        todo!("W1-MAP")
    }
    /// True for the shipped classic map (exempt from threshold scaling).
    pub fn is_classic(&self) -> bool {
        todo!("W1-MAP")
    }

    // --- geometry ---
    pub fn width(&self) -> i32 {
        todo!("W1-MAP")
    }
    pub fn height(&self) -> i32 {
        todo!("W1-MAP")
    }
    pub fn cell(&self, t: TilePos) -> Cell {
        let _ = t;
        todo!("W1-MAP")
    }
    /// Walkable for normal actors (not Wall, not Door, not House-only rules).
    pub fn walkable(&self, t: TilePos) -> bool {
        let _ = t;
        todo!("W1-MAP")
    }
    /// Warp a pixel position that has left the grid (tunnels); identity otherwise.
    pub fn warp(&self, p: PxPos) -> PxPos {
        let _ = p;
        todo!("W1-MAP")
    }
    /// Tiles where ghosts may not decide to turn Up (classic "red zones").
    pub fn no_up_tiles(&self) -> &[TilePos] {
        todo!("W1-MAP")
    }
    /// Tunnel-slowdown tiles.
    pub fn is_tunnel(&self, t: TilePos) -> bool {
        let _ = t;
        todo!("W1-MAP")
    }

    // --- counts ---
    /// Total collectibles (dots + energizers) at level start.
    pub fn pellets_total(&self) -> u32 {
        todo!("W1-MAP")
    }
    /// Dots only (for Elroy/fruit/counter scaling on custom maps).
    pub fn dots_total(&self) -> u32 {
        todo!("W1-MAP")
    }
    /// Map tile index for pellet bitmaps: y * width + x.
    pub fn tile_index(&self, t: TilePos) -> usize {
        let _ = t;
        todo!("W1-MAP")
    }

    // --- special locations ---
    pub fn pac_spawn(&self) -> PxPos {
        todo!("W1-MAP")
    }
    pub fn ghost_spawn(&self, g: GhostId) -> PxPos {
        let _ = g;
        todo!("W1-MAP")
    }
    pub fn ghost_spawn_dir(&self, g: GhostId) -> Dir {
        let _ = g;
        todo!("W1-MAP")
    }
    pub fn scatter_target(&self, g: GhostId) -> TilePos {
        let _ = g;
        todo!("W1-MAP")
    }
    /// Tile eyes navigate to before entering the house (above the door).
    pub fn eyes_target(&self) -> TilePos {
        todo!("W1-MAP")
    }
    /// Door tiles (Leaving/Entering ghosts cross these).
    pub fn door_tiles(&self) -> &[TilePos] {
        todo!("W1-MAP")
    }
    /// House interior center (revival point) in pixels.
    pub fn house_center(&self) -> PxPos {
        todo!("W1-MAP")
    }
    pub fn fruit_pos(&self) -> PxPos {
        todo!("W1-MAP")
    }
}
