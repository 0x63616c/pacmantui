//! Map format parsing, validation, and geometry queries.
//!
//! Format spec: docs/map-format.md (owned by W1-MAP together with this module).
//! Shipped maps live in `maps/` and are embedded via `include_str!`.
//!
//! Owner: W1-MAP agent. Public signatures are the contract; extend, don't break.

use std::collections::VecDeque;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::types::{Dir, Fix8, GhostId, PxPos, TILE_PX, TilePos};

const CLASSIC_SRC: &str = include_str!("../../maps/classic.pmtoml");
const CUSTOM_SRC: &str = include_str!("../../maps/custom.pmtoml");

/// Classic pellet total (dots + energizers); the denominator for dot-threshold
/// scaling on custom maps (docs/map-format.md "Rule adaptations").
const CLASSIC_PELLET_REFERENCE: u64 = 244;

/// Maximum supported grid dimension (tiles) per axis.
const MAX_DIM: usize = 256;

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

impl Cell {
    fn from_char(c: char) -> Option<Cell> {
        Some(match c {
            '#' => Cell::Wall,
            ' ' | '_' => Cell::Path,
            '.' => Cell::Dot,
            'o' => Cell::Energizer,
            '-' => Cell::Door,
            'H' => Cell::House,
            'T' => Cell::Tunnel,
            _ => return None,
        })
    }

    fn describe(self) -> &'static str {
        match self {
            Cell::Wall => "a wall",
            Cell::Path => "a path",
            Cell::Dot => "a dot",
            Cell::Energizer => "an energizer",
            Cell::Door => "a door",
            Cell::Tunnel => "a tunnel",
            Cell::House => "house interior",
        }
    }

    /// Walkable for normal actors (Pac-Man, Active ghosts).
    fn is_walkable(self) -> bool {
        matches!(
            self,
            Cell::Path | Cell::Dot | Cell::Energizer | Cell::Tunnel
        )
    }

    /// Traversable by ghosts with door access (Leaving/Entering/Eyes routing).
    fn is_ghost_traversable(self) -> bool {
        self.is_walkable() || matches!(self, Cell::Door | Cell::House)
    }
}

#[derive(Debug, Clone)]
pub struct Map {
    name: String,
    id: String,
    classic: bool,
    width: i32,
    height: i32,
    cells: Vec<Cell>,
    dots: u32,
    energizers: u32,
    pac_spawn: PxPos,
    pac_dir: Dir,
    ghost_spawns: [PxPos; 4],
    ghost_dirs: [Dir; 4],
    scatter: [TilePos; 4],
    eyes_target: TilePos,
    door_tiles: Vec<TilePos>,
    house_center: PxPos,
    fruit_pos: PxPos,
    no_up: Vec<TilePos>,
    threshold_scale_override: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    Io(String),
    Syntax {
        line: usize,
        msg: String,
    },
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

// ---------------------------------------------------------------------------
// TOML document schema (format_version = 1); see docs/map-format.md.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapDoc {
    // Checked via the loose pre-parse in `parse`; kept for schema completeness.
    #[allow(dead_code)]
    format_version: u32,
    name: String,
    id: String,
    grid: String,
    #[serde(default)]
    no_up_tiles: Vec<[i32; 2]>,
    spawns: SpawnsDoc,
    scatter: ScatterDoc,
    house: HouseDoc,
    fruit: FruitDoc,
    #[serde(default)]
    rules: RulesDoc,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpawnsDoc {
    pac: SpawnDoc,
    blinky: SpawnDoc,
    pinky: SpawnDoc,
    inky: SpawnDoc,
    clyde: SpawnDoc,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpawnDoc {
    tile: [i32; 2],
    #[serde(default)]
    offset_px: [i32; 2],
    facing: FacingDoc,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum FacingDoc {
    Up,
    Left,
    Down,
    Right,
}

impl From<FacingDoc> for Dir {
    fn from(f: FacingDoc) -> Dir {
        match f {
            FacingDoc::Up => Dir::Up,
            FacingDoc::Left => Dir::Left,
            FacingDoc::Down => Dir::Down,
            FacingDoc::Right => Dir::Right,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScatterDoc {
    blinky: [i32; 2],
    pinky: [i32; 2],
    inky: [i32; 2],
    clyde: [i32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HouseDoc {
    center: PointDoc,
    eyes_target: [i32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PointDoc {
    tile: [i32; 2],
    #[serde(default)]
    offset_px: [i32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FruitDoc {
    pos: PointDoc,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RulesDoc {
    threshold_scale: Option<f64>,
}

fn ghost_index(g: GhostId) -> usize {
    match g {
        GhostId::Blinky => 0,
        GhostId::Pinky => 1,
        GhostId::Inky => 2,
        GhostId::Clyde => 3,
    }
}

const GHOST_NAMES: [&str; 4] = ["blinky", "pinky", "inky", "clyde"];

fn syntax_error(src: &str, e: &toml::de::Error) -> MapError {
    let line = e
        .span()
        .map(|s| {
            src[..s.start.min(src.len())]
                .bytes()
                .filter(|&b| b == b'\n')
                .count()
                + 1
        })
        .unwrap_or(0);
    MapError::Syntax {
        line,
        msg: e.message().to_string(),
    }
}

fn invalid(msg: impl Into<String>) -> MapError {
    MapError::Invalid(msg.into())
}

/// FNV-1a 64-bit hash, used for the content-hash half of [`Map::id`].
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

impl Map {
    /// Parse and fully validate a map document.
    ///
    /// Validation order (first failure wins): TOML syntax -> `format_version`
    /// -> schema -> metadata -> grid shape/chars -> board structure (dots,
    /// door, house, seal, edges) -> spawns/points -> scatter/no-up -> warp
    /// pairing -> pellet reachability -> ghost routes. Every rule is
    /// documented in docs/map-format.md ("Validation").
    pub fn parse(src: &str) -> Result<Map, MapError> {
        // Loose pre-parse so an unsupported version is reported as such even
        // when the rest of the document doesn't match the v1 schema.
        let table: toml::Table = toml::from_str(src).map_err(|e| syntax_error(src, &e))?;
        match table.get("format_version") {
            None => return Err(invalid("missing required key `format_version`")),
            Some(v) => match v.as_integer() {
                None => return Err(invalid("`format_version` must be an integer")),
                Some(1) => {}
                Some(v) => {
                    return Err(MapError::UnsupportedVersion(
                        u32::try_from(v).unwrap_or(u32::MAX),
                    ));
                }
            },
        }
        let doc: MapDoc = toml::from_str(src).map_err(|e| syntax_error(src, &e))?;
        Map::build(doc, src)
    }

    fn build(doc: MapDoc, src: &str) -> Result<Map, MapError> {
        // --- metadata ---
        if doc.name.trim().is_empty() {
            return Err(invalid("map name must not be empty"));
        }
        if doc.id.is_empty() {
            return Err(invalid("map id must not be empty"));
        }
        if !doc
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(invalid(format!(
                "map id {:?} may only contain lowercase ASCII letters, digits, '-' and '_'",
                doc.id
            )));
        }
        if let Some(s) = doc.rules.threshold_scale
            && !(s.is_finite() && s > 0.0)
        {
            return Err(invalid("`rules.threshold_scale` must be finite and > 0"));
        }

        // --- grid shape and characters ---
        let grid_str = doc.grid.strip_suffix('\n').unwrap_or(&doc.grid);
        let rows: Vec<&str> = grid_str
            .split('\n')
            .map(|r| r.strip_suffix('\r').unwrap_or(r))
            .collect();
        if grid_str.is_empty() {
            return Err(invalid("grid is empty"));
        }
        let width = rows[0].chars().count();
        let height = rows.len();
        if width == 0 {
            return Err(invalid("grid row 0 is empty"));
        }
        if width > MAX_DIM || height > MAX_DIM {
            return Err(invalid(format!(
                "grid is {width}x{height}; maximum supported size is {MAX_DIM}x{MAX_DIM}"
            )));
        }
        let mut cells = Vec::with_capacity(width * height);
        for (y, row) in rows.iter().enumerate() {
            let n = row.chars().count();
            if n != width {
                return Err(invalid(format!(
                    "grid row {y} has length {n}, expected {width} (grid must be rectangular)"
                )));
            }
            for (x, c) in row.chars().enumerate() {
                let cell = Cell::from_char(c).ok_or_else(|| {
                    invalid(format!(
                        "grid row {y}, col {x}: illegal character {c:?} \
                         (legend: '#' ' ' '_' '.' 'o' '-' 'H' 'T')"
                    ))
                })?;
                cells.push(cell);
            }
        }
        let (w, h) = (width as i32, height as i32);
        let cell_at = |t: TilePos| -> Cell {
            if t.x < 0 || t.y < 0 || t.x >= w || t.y >= h {
                Cell::Wall
            } else {
                cells[(t.y * w + t.x) as usize]
            }
        };

        // --- board structure ---
        let mut dots = 0u32;
        let mut energizers = 0u32;
        let mut door_tiles = Vec::new();
        let mut house_tiles = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let t = TilePos::new(x, y);
                match cell_at(t) {
                    Cell::Dot => dots += 1,
                    Cell::Energizer => energizers += 1,
                    Cell::Door => door_tiles.push(t),
                    Cell::House => house_tiles.push(t),
                    _ => {}
                }
            }
        }
        if dots == 0 {
            return Err(invalid(
                "map has no dots (at least one '.' tile is required)",
            ));
        }
        if door_tiles.is_empty() {
            return Err(invalid("map has no door ('-') tiles"));
        }
        if house_tiles.is_empty() {
            return Err(invalid("map has no ghost-house interior ('H') tiles"));
        }
        for &t in &door_tiles {
            if t.x == 0 || t.y == 0 || t.x == w - 1 || t.y == h - 1 {
                return Err(invalid(format!(
                    "door tile ({}, {}) lies on the grid edge",
                    t.x, t.y
                )));
            }
            let adj_house = Dir::IN_PRIORITY_ORDER
                .iter()
                .any(|d| cell_at(step(t, *d)) == Cell::House);
            if !adj_house {
                return Err(invalid(format!(
                    "door tile ({}, {}) is not adjacent to any house interior tile",
                    t.x, t.y
                )));
            }
        }
        for &t in &house_tiles {
            if t.x == 0 || t.y == 0 || t.x == w - 1 || t.y == h - 1 {
                return Err(invalid(format!(
                    "house interior tile ({}, {}) lies on the grid edge",
                    t.x, t.y
                )));
            }
            for d in Dir::IN_PRIORITY_ORDER {
                let n = step(t, d);
                let c = cell_at(n);
                if !matches!(c, Cell::Wall | Cell::House | Cell::Door) {
                    return Err(invalid(format!(
                        "house interior tile ({}, {}) opens onto ({}, {}) without a door \
                         (the house must be sealed by walls and door tiles)",
                        t.x, t.y, n.x, n.y
                    )));
                }
            }
        }

        // --- spawns and pixel points ---
        let resolve = |what: &str, tile: [i32; 2], offset: [i32; 2]| -> Result<PxPos, MapError> {
            let [tx, ty] = tile;
            if tx < 0 || ty < 0 || tx >= w || ty >= h {
                return Err(invalid(format!(
                    "{what}: tile ({tx}, {ty}) is outside the {w}x{h} grid"
                )));
            }
            let [ox, oy] = offset;
            if !(-4..=4).contains(&ox) || !(-4..=4).contains(&oy) {
                return Err(invalid(format!(
                    "{what}: offset_px [{ox}, {oy}] out of range (each component must be in -4..=4)"
                )));
            }
            let (px, py) = (
                tx * TILE_PX + TILE_PX / 2 + ox,
                ty * TILE_PX + TILE_PX / 2 + oy,
            );
            if px < 0 || py < 0 || px >= w * TILE_PX || py >= h * TILE_PX {
                return Err(invalid(format!(
                    "{what}: pixel position ({px}, {py}) is outside the grid"
                )));
            }
            Ok(PxPos {
                x: Fix8::from_px(px),
                y: Fix8::from_px(py),
            })
        };

        let pac_spawn = resolve("pac spawn", doc.spawns.pac.tile, doc.spawns.pac.offset_px)?;
        let pac_tile = pac_spawn.tile();
        if !cell_at(pac_tile).is_walkable() {
            return Err(invalid(format!(
                "pac spawn occupies tile ({}, {}) which is {}",
                pac_tile.x,
                pac_tile.y,
                cell_at(pac_tile).describe()
            )));
        }

        let ghost_docs = [
            &doc.spawns.blinky,
            &doc.spawns.pinky,
            &doc.spawns.inky,
            &doc.spawns.clyde,
        ];
        let mut ghost_spawns = [PxPos::default(); 4];
        let mut ghost_dirs = [Dir::Left; 4];
        for (i, g) in ghost_docs.iter().enumerate() {
            let name = GHOST_NAMES[i];
            let pos = resolve(&format!("{name} spawn"), g.tile, g.offset_px)?;
            let t = pos.tile();
            let c = cell_at(t);
            if !(c.is_walkable() || c == Cell::House) {
                return Err(invalid(format!(
                    "{name} spawn occupies tile ({}, {}) which is {} \
                     (ghosts must spawn on walkable or house tiles)",
                    t.x,
                    t.y,
                    c.describe()
                )));
            }
            if t == pac_tile {
                return Err(invalid(format!(
                    "pac spawn tile ({}, {}) overlaps the {name} spawn",
                    t.x, t.y
                )));
            }
            ghost_spawns[i] = pos;
            ghost_dirs[i] = g.facing.into();
        }

        let house_center = resolve(
            "house center",
            doc.house.center.tile,
            doc.house.center.offset_px,
        )?;
        let hc_tile = house_center.tile();
        if cell_at(hc_tile) != Cell::House {
            return Err(invalid(format!(
                "house center ({}, {}) is not inside the house interior",
                hc_tile.x, hc_tile.y
            )));
        }

        let eyes_target = TilePos::new(doc.house.eyes_target[0], doc.house.eyes_target[1]);
        if !cell_at(eyes_target).is_walkable() {
            return Err(invalid(format!(
                "eyes target ({}, {}) is not a walkable board tile",
                eyes_target.x, eyes_target.y
            )));
        }

        let fruit_pos = resolve(
            "fruit position",
            doc.fruit.pos.tile,
            doc.fruit.pos.offset_px,
        )?;
        let fruit_tile = fruit_pos.tile();
        if !cell_at(fruit_tile).is_walkable() {
            return Err(invalid(format!(
                "fruit position occupies tile ({}, {}) which is not a walkable tile",
                fruit_tile.x, fruit_tile.y
            )));
        }

        // --- scatter targets (may be off-grid, but must be sane) ---
        let scatter_docs = [
            doc.scatter.blinky,
            doc.scatter.pinky,
            doc.scatter.inky,
            doc.scatter.clyde,
        ];
        let mut scatter = [TilePos::default(); 4];
        for (i, s) in scatter_docs.iter().enumerate() {
            let t = TilePos::new(s[0], s[1]);
            if t.x < -w || t.x > 2 * w || t.y < -h || t.y > 2 * h {
                return Err(invalid(format!(
                    "{} scatter target ({}, {}) is out of range \
                     (allowed: x in {}..={}, y in {}..={})",
                    GHOST_NAMES[i],
                    t.x,
                    t.y,
                    -w,
                    2 * w,
                    -h,
                    2 * h
                )));
            }
            scatter[i] = t;
        }

        // --- no-up tiles ---
        let mut no_up = Vec::with_capacity(doc.no_up_tiles.len());
        for nu in &doc.no_up_tiles {
            let t = TilePos::new(nu[0], nu[1]);
            if !cell_at(t).is_walkable() {
                return Err(invalid(format!(
                    "no-up tile ({}, {}) is not a walkable tile",
                    t.x, t.y
                )));
            }
            if no_up.contains(&t) {
                return Err(invalid(format!("duplicate no-up tile ({}, {})", t.x, t.y)));
            }
            no_up.push(t);
        }

        // --- warp pairing: walkable edge tiles must pair with the opposite edge ---
        let pair_check = |a: TilePos, b: TilePos, edge: &str| -> Result<(), MapError> {
            if cell_at(a).is_walkable() && !cell_at(b).is_walkable() {
                return Err(invalid(format!(
                    "unpaired warp tile: ({}, {}) on the {edge} edge is walkable but its \
                     opposite-edge partner ({}, {}) is not",
                    a.x, a.y, b.x, b.y
                )));
            }
            Ok(())
        };
        for y in 0..h {
            pair_check(TilePos::new(0, y), TilePos::new(w - 1, y), "left")?;
            pair_check(TilePos::new(w - 1, y), TilePos::new(0, y), "right")?;
        }
        for x in 0..w {
            pair_check(TilePos::new(x, 0), TilePos::new(x, h - 1), "top")?;
            pair_check(TilePos::new(x, h - 1), TilePos::new(x, 0), "bottom")?;
        }

        // --- reachability ---
        // Pac-Man graph: walkable tiles only (doors and the house block him).
        let pac_reach = flood(w, h, pac_tile, |t| cell_at(t).is_walkable());
        for y in 0..h {
            for x in 0..w {
                let t = TilePos::new(x, y);
                let c = cell_at(t);
                if matches!(c, Cell::Dot | Cell::Energizer) && !pac_reach[(y * w + x) as usize] {
                    let what = if c == Cell::Dot { "dot" } else { "energizer" };
                    return Err(invalid(format!(
                        "{what} at ({x}, {y}) is unreachable from the pac spawn \
                         (Pac-Man cannot cross doors or the house)"
                    )));
                }
            }
        }

        // Ghost graph: walkable + door + house (Leaving/Entering door access).
        let ghost_reach = flood(w, h, hc_tile, |t| cell_at(t).is_ghost_traversable());
        let reached = |t: TilePos| ghost_reach[(t.y * w + t.x) as usize];
        if !reached(eyes_target) {
            return Err(invalid(format!(
                "no ghost route between the house center ({}, {}) and the eyes target \
                 ({}, {}) through the door (ghosts could never leave, eyes could never return)",
                hc_tile.x, hc_tile.y, eyes_target.x, eyes_target.y
            )));
        }
        for (i, pos) in ghost_spawns.iter().enumerate() {
            let t = pos.tile();
            let (ok, from) = if cell_at(t) == Cell::House {
                (reached(t), "the house center")
            } else {
                (pac_reach[(t.y * w + t.x) as usize], "the pac spawn")
            };
            if !ok {
                return Err(invalid(format!(
                    "{} spawn tile ({}, {}) is not reachable from {from}",
                    GHOST_NAMES[i], t.x, t.y
                )));
            }
        }

        let id = format!("{}-{:016x}", doc.id, fnv1a64(src.as_bytes()));
        Ok(Map {
            name: doc.name,
            id,
            classic: src == CLASSIC_SRC,
            width: w,
            height: h,
            cells,
            dots,
            energizers,
            pac_spawn,
            pac_dir: doc.spawns.pac.facing.into(),
            ghost_spawns,
            ghost_dirs,
            scatter,
            eyes_target,
            door_tiles,
            house_center,
            fruit_pos,
            no_up,
            threshold_scale_override: doc.rules.threshold_scale,
        })
    }

    /// The embedded classic map (must parse+validate; covered by tests).
    pub fn classic() -> Map {
        static CACHE: OnceLock<Map> = OnceLock::new();
        CACHE
            .get_or_init(|| Map::parse(CLASSIC_SRC).expect("embedded classic map must be valid"))
            .clone()
    }

    /// The embedded custom map.
    pub fn custom() -> Map {
        static CACHE: OnceLock<Map> = OnceLock::new();
        CACHE
            .get_or_init(|| Map::parse(CUSTOM_SRC).expect("embedded custom map must be valid"))
            .clone()
    }

    pub fn load_file(path: &std::path::Path) -> Result<Map, MapError> {
        let src = std::fs::read_to_string(path)
            .map_err(|e| MapError::Io(format!("{}: {e}", path.display())))?;
        Map::parse(&src)
    }

    // --- identity ---
    /// Display name (from metadata).
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Stable id for score separation: metadata id + content hash
    /// (`<id>-<16 hex digits>`, FNV-1a 64 over the source document).
    pub fn id(&self) -> &str {
        &self.id
    }
    /// True for the shipped classic map (exempt from threshold scaling).
    pub fn is_classic(&self) -> bool {
        self.classic
    }

    // --- geometry ---
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    /// Cell at `t`; anything outside the grid reads as [`Cell::Wall`].
    pub fn cell(&self, t: TilePos) -> Cell {
        if t.x < 0 || t.y < 0 || t.x >= self.width || t.y >= self.height {
            Cell::Wall
        } else {
            self.cells[(t.y * self.width + t.x) as usize]
        }
    }
    /// Walkable for normal actors (not Wall, not Door, not House-only rules).
    pub fn walkable(&self, t: TilePos) -> bool {
        self.cell(t).is_walkable()
    }
    /// Warp a pixel position that has left the grid (tunnels); identity otherwise.
    ///
    /// Wraps modulo the grid's pixel size on both axes; validation guarantees
    /// walkable edge tiles pair up, so a legal exit re-enters on the matching
    /// opposite-edge tile.
    pub fn warp(&self, p: PxPos) -> PxPos {
        let w = self.width * TILE_PX * 256;
        let h = self.height * TILE_PX * 256;
        PxPos {
            x: Fix8(p.x.raw().rem_euclid(w)),
            y: Fix8(p.y.raw().rem_euclid(h)),
        }
    }
    /// Tiles where ghosts may not decide to turn Up (classic "red zones").
    pub fn no_up_tiles(&self) -> &[TilePos] {
        &self.no_up
    }
    /// Tunnel-slowdown tiles.
    pub fn is_tunnel(&self, t: TilePos) -> bool {
        self.cell(t) == Cell::Tunnel
    }

    // --- counts ---
    /// Total collectibles (dots + energizers) at level start.
    pub fn pellets_total(&self) -> u32 {
        self.dots + self.energizers
    }
    /// Dots only (for Elroy/fruit/counter scaling on custom maps).
    pub fn dots_total(&self) -> u32 {
        self.dots
    }
    /// Map tile index for pellet bitmaps: y * width + x.
    pub fn tile_index(&self, t: TilePos) -> usize {
        debug_assert!(
            t.x >= 0 && t.y >= 0 && t.x < self.width && t.y < self.height,
            "tile_index out of bounds: ({}, {})",
            t.x,
            t.y
        );
        (t.y * self.width + t.x) as usize
    }

    /// Scale a classic dot-count threshold (Elroy dots, fruit triggers, house
    /// counters) to this map: `round(value * pellets_total / 244)`, min 0.
    /// The classic map returns the value unchanged; `[rules] threshold_scale`
    /// overrides the ratio (docs/map-format.md "Rule adaptations").
    pub fn scale_dot_threshold(&self, classic_value: u32) -> u32 {
        if self.classic {
            return classic_value;
        }
        if let Some(s) = self.threshold_scale_override {
            // s is validated finite and > 0; result clamped to u32 range.
            return (f64::from(classic_value) * s)
                .round()
                .clamp(0.0, f64::from(u32::MAX)) as u32;
        }
        let p = u64::from(self.pellets_total());
        // round(classic_value * p / 244) in integer arithmetic.
        ((u64::from(classic_value) * p * 2 + CLASSIC_PELLET_REFERENCE)
            / (2 * CLASSIC_PELLET_REFERENCE)) as u32
    }

    // --- special locations ---
    pub fn pac_spawn(&self) -> PxPos {
        self.pac_spawn
    }
    /// Pac-Man's initial facing direction.
    pub fn pac_spawn_dir(&self) -> Dir {
        self.pac_dir
    }
    pub fn ghost_spawn(&self, g: GhostId) -> PxPos {
        self.ghost_spawns[ghost_index(g)]
    }
    pub fn ghost_spawn_dir(&self, g: GhostId) -> Dir {
        self.ghost_dirs[ghost_index(g)]
    }
    pub fn scatter_target(&self, g: GhostId) -> TilePos {
        self.scatter[ghost_index(g)]
    }
    /// Tile eyes navigate to before entering the house (above the door).
    pub fn eyes_target(&self) -> TilePos {
        self.eyes_target
    }
    /// Door tiles (Leaving/Entering ghosts cross these).
    pub fn door_tiles(&self) -> &[TilePos] {
        &self.door_tiles
    }
    /// House interior center (revival point) in pixels.
    pub fn house_center(&self) -> PxPos {
        self.house_center
    }
    pub fn fruit_pos(&self) -> PxPos {
        self.fruit_pos
    }
}

/// One tile step in direction `d` (no wrapping).
fn step(t: TilePos, d: Dir) -> TilePos {
    let (dx, dy) = d.delta();
    TilePos::new(t.x + dx, t.y + dy)
}

/// BFS flood fill over tiles passing `open`, with edge-warp adjacency
/// (stepping off one edge continues on the opposite edge, matching `warp`).
/// Returns a row-major visited bitmap; a closed start tile yields all-false.
fn flood(w: i32, h: i32, start: TilePos, open: impl Fn(TilePos) -> bool) -> Vec<bool> {
    let mut seen = vec![false; (w * h) as usize];
    if !open(start) {
        return seen;
    }
    seen[(start.y * w + start.x) as usize] = true;
    let mut queue = VecDeque::from([start]);
    while let Some(t) = queue.pop_front() {
        for d in Dir::IN_PRIORITY_ORDER {
            let s = step(t, d);
            let n = TilePos::new(s.x.rem_euclid(w), s.y.rem_euclid(h));
            let i = (n.y * w + n.x) as usize;
            if !seen[i] && open(n) {
                seen[i] = true;
                queue.push_back(n);
            }
        }
    }
    seen
}
