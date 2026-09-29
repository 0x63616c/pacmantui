# pacmantui map format (`.pmtoml`), format_version 1

Owner: W1-MAP (spec + `src/map/` implement each other; tests in
`tests/map_*.rs`). Shipped maps: `maps/classic.pmtoml`, `maps/custom.pmtoml`
(embedded into the binary via `include_str!`).

A map is a single TOML document: metadata keys, one multi-line grid string,
and a handful of tables for spawns and special locations. The parser
(`Map::parse`) fully validates every map before the sim ever sees it; a map
that parses is guaranteed playable in the structural sense described under
[Validation](#validation).

## Versioning

```toml
format_version = 1
```

Required, must be the integer `1`. Any other integer is rejected with
`MapError::UnsupportedVersion(n)` (the rest of the document is not
interpreted). A missing key or a non-integer value is
`MapError::Invalid`. Backward-incompatible format changes bump the version.

## Coordinate model

- The grid is `width x height` tiles of 8x8 px (`TILE_PX`); tile `(0,0)` is
  top-left, x grows right, y grows down. Tile `(x,y)` covers pixels
  `8x..8x+7`, `8y..8y+7`; the tile center pixel is `(8x+4, 8y+4)`.
- Dimensions are implied by the grid string (row count x row length). There
  are no width/height keys.
- An actor occupies the tile containing its pixel center
  (`floor(center/8)`, dossier convention).
- **HUD vs playfield is not map data.** The renderer decides margins; the
  classic map simply includes the arcade's HUD rows (0-2, 34-35) as wall
  rows so tile coordinates match the dossier's 28x36 screen grid. Custom
  maps need no such rows.

## Metadata

```toml
name = "Classic"   # display name; non-empty
id = "classic"     # stable id; [a-z0-9_-]+, non-empty
```

`Map::id()` returns `<id>-<16 hex digits>`, where the hash is FNV-1a 64 over
the exact source text. High-score tables key on this, so editing a map file
separates its scores. `Map::is_classic()` is true only when the source is
byte-identical to the shipped `maps/classic.pmtoml`.

## Grid

```toml
grid = '''
###########
#o.......o#
#.###-###.#
...
'''
```

One line per tile row. A single trailing newline (from the closing `'''`)
is ignored; a trailing `\r` per line is tolerated (CRLF checkouts).

| Char | Cell | Meaning |
|------|------|---------|
| `#` | `Wall` | dead space; never enterable |
| `.` | `Dot` | walkable, holds a dot (10 pts) |
| `o` | `Energizer` | walkable, holds an energizer (50 pts) |
| ` ` or `_` | `Path` | walkable, no collectible |
| `T` | `Tunnel` | walkable, no collectible, ghost-slowdown zone |
| `-` | `Door` | ghost-house door; crossable only by Leaving/Entering/Eyes ghosts |
| `H` | `House` | ghost-house interior floor; only housed ghosts occupy it |

Prefer `_` over a space in a row's final column so editors that strip
trailing whitespace cannot corrupt the grid. Anything outside the grid reads
as `Wall` (`Map::cell` on out-of-bounds tiles), which is how scatter targets
in "dead space" behave.

**Walkable** (for Pac-Man and Active ghosts) means `Dot`, `Energizer`,
`Path`, or `Tunnel`. `Door` and `House` are traversable only by ghosts in
door-crossing states.

## Spawns

```toml
[spawns]
pac    = { tile = [13, 26], offset_px = [4, 0], facing = "left" }
blinky = { tile = [13, 14], offset_px = [4, 0], facing = "left" }
pinky  = { tile = [13, 17], offset_px = [4, 0], facing = "down" }
inky   = { tile = [11, 17], offset_px = [4, 0], facing = "up" }
clyde  = { tile = [15, 17], offset_px = [4, 0], facing = "up" }
```

All five spawns are required; `facing` is required (`"up" | "left" | "down"
| "right"`); `offset_px` defaults to `[0, 0]`.

Spawns are **pixel-precise**: `pixel = tile_center + offset_px`, each offset
component in `-4..=4`. Classic actors start centered on the boundary between
two tile columns (a 16x16 sprite over 8x8 tiles), which is expressed as the
left tile plus `offset_px = [4, 0]` — e.g. Pac-Man `tile [13,26] + [4,0]` =
pixel `(112, 212)`, whose occupied tile is `(14, 26)`.

A ghost whose occupied tile is `House` starts in the house (release
counters apply); one on a walkable tile starts on the board (classic
Blinky). Validation constrains where each may sit (below).

## Scatter targets

```toml
[scatter]
blinky = [25, -1]
pinky  = [2, -1]
inky   = [27, 34]
clyde  = [0, 34]
```

Tile coordinates, all four required. They **may lie off-grid** (the classic
ones do; off-grid tiles read as walls, making the target unreachable — that
looping pursuit *is* scatter behavior). They must be sane: each component
within `-dim ..= 2*dim` of its axis.

## House, eyes target, fruit

```toml
[house]
center = { tile = [13, 17], offset_px = [4, 0] }   # revival point (pixels)
eyes_target = [13, 14]                              # tile above the door

[fruit]
pos = { tile = [13, 20], offset_px = [4, 0] }       # fruit center (pixels)
```

`center` and `pos` use the same tile+offset pixel form as spawns. The house
center must resolve to a `House` tile; eaten eyes navigate to `eyes_target`
(a walkable board tile), then cross the door to the center. The fruit
position must resolve to a walkable tile.

## Tunnels and warping

Warping is defined by the **grid edges**, not by `T` tiles: a walkable tile
on the left/right/top/bottom edge is a warp mouth. When an actor's pixel
center leaves the grid, `Map::warp` wraps it modulo the grid's pixel size on
both axes — leaving via column 0 at row *y* re-enters at column `width-1`,
row *y* (and vice versa); leaving via the top edge at column *x* re-enters
at the bottom in the same column. Warp mouths must therefore **pair up**:
for every walkable edge tile, the tile at the same row (opposite horizontal
edge) or same column (opposite vertical edge) must also be walkable.

`T` tiles mark the ghost-slowdown zone (`Map::is_tunnel`); they are
otherwise ordinary dot-free walkable tiles and may appear anywhere (the
classic zone extends well inside the grid). Maps with zero warp mouths and
zero `T` tiles are legal.

## No-up tiles

```toml
no_up_tiles = [[12, 14], [15, 14], [12, 26], [15, 26]]
```

Optional (default empty). The classic "red zones": decision tiles where
ghosts in scatter/chase may not choose Up. Each must be a walkable tile;
duplicates are rejected.

## Rule adaptations (non-244-pellet maps)

Classic dot-count thresholds (Elroy 1/2 dots-remaining, fruit triggers at
70/170 dots, house dot-counter limits) assume 244 pellets. For other maps
they scale by the pellet ratio (IMPLEMENTATION.md, binding):

```
scaled = round(classic_value * pellets_total / 244), minimum 0
```

`Map::scale_dot_threshold(classic_value)` implements this (integer
arithmetic, round-half-up). The classic map bypasses scaling entirely
(`is_classic()`), returning values unchanged. A map may override the ratio:

```toml
[rules]
threshold_scale = 1.5   # optional; finite, > 0
```

When present, `scaled = round(classic_value * threshold_scale)` instead of
the pellet ratio.

## Validation

`Map::parse` enforces every rule below, in this order; the first failure is
returned. Errors use the `MapError` variants:

- `Syntax { line, msg }` — TOML-level problems: malformed TOML, unknown
  keys anywhere (`deny_unknown_fields`), missing required keys/tables
  (e.g. a missing ghost spawn reports `missing field 'pinky'`), wrong
  types, bad `facing` strings. `line` is the 1-based source line when the
  parser can locate it, else 0.
- `UnsupportedVersion(n)` — `format_version` is an integer other than 1.
- `Invalid(msg)` — semantic failures (below). Messages name the offending
  coordinates. There are no warnings: everything below is an error.
- `Io(msg)` — `Map::load_file` could not read the file (message includes
  the path).

Semantic rules (`Invalid`), grouped as checked:

1. **Metadata**: `name` non-empty; `id` non-empty and limited to
   `[a-z0-9_-]`; `rules.threshold_scale` (if present) finite and > 0.
2. **Grid shape**: non-empty; every row the same length (*"grid must be
   rectangular"*, naming the row); at most 256x256; only legend characters
   (*"illegal character"*, naming row/col).
3. **Board structure**: at least one dot; at least one door tile; at least
   one house tile; no door or house tile on a grid edge; every door tile
   adjacent (4-neighborhood) to a house tile; the house is **sealed** —
   every neighbor of a house tile is wall, house, or door (*"opens onto
   (x, y) without a door"*).
4. **Spawns/points**: every referenced tile inside the grid; every
   `offset_px` component in `-4..=4` and the resulting pixel inside the
   grid. Pac-Man's occupied tile must be walkable. Each ghost's occupied
   tile must be walkable or house (never wall/door). No ghost may share
   Pac-Man's occupied tile. House center on a `House` tile; eyes target
   walkable; fruit tile walkable.
5. **Scatter targets**: each component within `-dim ..= 2*dim`.
6. **No-up tiles**: each walkable; no duplicates.
7. **Warp pairing**: every walkable edge tile has a walkable partner on the
   opposite edge (same row / same column) — *"unpaired warp tile"*.
8. **Pellet reachability**: every dot and energizer reachable from
   Pac-Man's spawn tile through walkable tiles only — doors and the house
   block Pac-Man — with warp adjacency across paired edges.
9. **Ghost routes** (entity access modeled: the ghost graph adds `Door` and
   `House` to the walkable graph): the eyes target must be connected to the
   house center — since the house is sealed (rule 3), any such route
   necessarily crosses a door, proving both that housed ghosts can leave
   and that eyes can return. Each in-house ghost spawn must be connected to
   the house center; each outside ghost spawn must be reachable from
   Pac-Man's spawn on the walkable graph.

## Unsupported topology

Deliberately out of scope for format 1 (the validator has no way to express
these, and the sim assumes their absence):

- non-rectangular grids, or grids over 256x256;
- warps between non-matching positions (portals): edge exits always
  re-enter at the mirrored edge tile — no cross-row/column or interior
  teleports;
- one-way tiles or direction-restricted passages other than the no-up list
  (and the door's entity restriction);
- multiple ghost houses (multiple door tiles are fine, but all house tiles
  must belong to one connected region reachable from the single
  `house.center`; a second sealed house region would fail rule 9 for any
  ghost spawned there);
- moving/stateful geometry, per-level layout changes, more or fewer than
  four ghosts, more than one Pac-Man spawn (the schema fixes the actor
  set);
- dots or energizers inside the house or on doors (`H`/`-` cannot carry
  collectibles by construction);
- diagonal adjacency (all connectivity is 4-neighbor).

## Consumer API summary

`src/map/mod.rs` exposes the parsed map: identity (`name`, `id`,
`is_classic`), geometry (`width`, `height`, `cell`, `walkable`, `warp`,
`is_tunnel`, `no_up_tiles`, `tile_index`), counts (`dots_total`,
`pellets_total`, `scale_dot_threshold`), and locations (`pac_spawn`,
`pac_spawn_dir`, `ghost_spawn`, `ghost_spawn_dir`, `scatter_target`,
`eyes_target`, `door_tiles`, `house_center`, `fruit_pos`). `Map::classic()`
/ `Map::custom()` return the embedded maps; `Map::load_file` loads external
`.pmtoml` files.
