# Pac-Man Game Mechanics — Specification from The Pac-Man Dossier

Research notes for `pacmantui`. Every rule below is sourced from *The Pac-Man Dossier* by
Jamey Pittman. Citations name the dossier chapter/section that states the rule.

**Sources used**

- HTML mirror: <https://pacman.holenet.info/> — single page, version **1.0.27, August 11, 2015**.
  This is the newer, complete text and the authoritative cross-check.
- PDF: "The Pac-Man Dossier.pdf" (tralvex.com mirror) — a January 2015 print of the older
  v1.0.26 page ("48 pages"; the file is missing printed page 12, and its Table A.1 header
  lacks the `100% speed` annotation that v1.0.27 added). Where the two differ, the HTML
  (v1.0.27) was used. Page images from the PDF were used to read the diagrams.
- Diagram coordinates below were obtained by pixel-measuring the dossier's own diagram
  images (`Tiles.png`, `lvl1.png`, `Scatter.png`, `exploit.png`, `StartPositions.png` as
  served by the HTML mirror — the same images embedded in the PDF), calibrated against the
  known 8x8 tile grid. The measurement method and any residual doubt are recorded in
  "Ambiguities & resolutions" at the end.

**Out of scope here:** full transcriptions of Appendix A Table A.1 (Level Specifications)
and Table A.2 (Difficulty Specifications) — owned by another research task. Sections below
mark every value that lives in those tables with "→ Table A.1" / "→ Table A.2".

**Coordinate convention used throughout this document:** `(col, row)` tile coordinates,
`col` 0–27 left→right, `row` 0–35 top→bottom, over the full 28x36-tile screen. Pixel
coordinates are screen pixels, `(0,0)` top-left, 224x288. Tile `(c,r)` covers pixels
`x = 8c .. 8c+7`, `y = 8r .. 8r+7`.

---

## 1. Board & coordinates

### 1.1 Screen and tile grid

- Screen is 224 x 288 pixels; tiles are 8x8 px, giving a **28 x 36 tile grid**.
  (Ch. 3 "Maze Logic 101": "The visible game screen should be thought of as a regular grid
  of tiles, each eight pixels square. The actual pixel dimensions of the screen are
  224 x 288, so dividing each value by eight yields a grid that is 28 x 36 tiles in size.")
- Hardware resolution 224x288 confirmed in Appendix C "Specifications".
- Every tile is either **legal space** (actors can occupy it) or **dead space** (Ch. 3).
  Measured from the dossier's `Tiles.png` legal-space diagram: there are exactly
  **300 legal tiles** (including the 4 energizer tiles).
- **HUD rows** (dead space, no maze): rows **0–2** at the top (1UP / HIGH SCORE / score
  digits) and rows **34–35** at the bottom (remaining lives at bottom-left, fruit/level
  counter at bottom-right — Ch. 2 "The Basics" mentions the level counter "along the
  bottom edge of the screen"). The maze walls occupy rows **3–33**, i.e. the playfield is
  **28 x 31 tiles** (verified by measuring `Tiles.png`: wall pixels span y = 24..271).
- Each dot sits in the center of its tile, so dots are exactly 8 px (one tile) apart
  (Ch. 3 "Maze Logic 101").

### 1.2 The maze (level 1 layout — identical every level)

Transcribed programmatically from the dossier's own `Tiles.png` (legal space) and
`lvl1.png` (dots/energizers) diagrams; counts self-verify: **240 dots, 4 energizers,
56 legal tiles with no dot**. Grid below is the full 28x36 screen.

Legend: `#` wall/dead space · `.` dot (10 pts) · `o` energizer (50 pts) ·
(space) legal path tile with no dot · `=` ghost-house door tile · `H` house interior
(not legal space for normal routing).

```text
     0000000000111111111122222222
     0123456789012345678901234567
 0   ############################   <- HUD
 1   ############################   <- HUD
 2   ############################   <- HUD
 3   ############################   <- top wall row
 4   #............##............#
 5   #.####.#####.##.#####.####.#
 6   #o####.#####.##.#####.####o#   <- energizers (1,6) (26,6)
 7   #.####.#####.##.#####.####.#
 8   #..........................#
 9   #.####.##.########.##.####.#
10   #.####.##.########.##.####.#
11   #......##....##....##......#
12   ######.##### ## #####.######   <- no-dot stubs (12,12) (15,12)
13   ######.##### ## #####.######
14   ######.##          ##.######   <- Blinky start row; corridor (9..18,14)
15   ######.## ###==### ##.######   <- door tiles (13,15) (14,15)
16   ######.## #HHHHHH# ##.######
17         .   #HHHHHH#   .          <- TUNNEL row (wraps col 0 <-> col 27)
18   ######.## #HHHHHH# ##.######
19   ######.## ######## ##.######
20   ######.##          ##.######   <- fruit appears here (below the pen)
21   ######.## ######## ##.######
22   ######.## ######## ##.######
23   #............##............#
24   #.####.#####.##.#####.####.#
25   #.####.#####.##.#####.####.#
26   #o..##.......  .......##..o#   <- energizers (1,26) (26,26); Pac-Man start (13/14,26)
27   ###.##.##.########.##.##.###
28   ###.##.##.########.##.##.###
29   #......##....##....##......#
30   #.##########.##.##########.#
31   #.##########.##.##########.#
32   #..........................#
33   ############################   <- bottom wall row
34   ############################   <- HUD (lives / fruit counter)
35   ############################   <- HUD
```

- **244 dots total**: 240 small dots (10 pts) + 4 energizers (50 pts) at
  `(1,6) (26,6) (1,26) (26,26)` (Ch. 2 "The Basics"; positions measured from `lvl1.png`).
  Clearing all 244 = 2,600 points and ends the level.
- **Paths without dots** (56 tiles, measured; all verified against `lvl1.png`):
  - top-of-house stubs: `(12,12) (15,12) (12,13) (15,13)`
  - corridor over the house: `(9..18, 14)`
  - house flanks: `(9,15) (18,15) (9,16) (18,16) (9,18) (18,18) (9,19) (18,19)`
  - tunnel row 17: `(0..5,17) (7,17) (8,17) (18..20,17) (22..27,17)` — the only dotted
    tiles on row 17 are `(6,17)` and `(21,17)`
  - corridor under the house: `(9..18, 20)`, plus `(9,21) (18,21) (9,22) (18,22)`
  - Pac-Man's start tiles: `(13,26) (14,26)`
- **Tunnel** ("side tunnel", Glossary): row 17 wraps between the left and right screen
  edges. Slow-down zones for ghosts, see §2.5/§4.13.
- **Ghost house** ("monster pen", Ch. 2 "Home Sweet Home"): rectangular structure at
  cols 10–17, rows 15–19; interior floor cols 11–16, rows 16–18; the pink **door** on top
  occupies tiles `(13,15)` and `(14,15)` (measured in both `Tiles.png` and `Scatter.png`).
  The house is off-limits to Pac-Man. A ghost that has left cannot re-enter except as
  eaten eyes (Ch. 2 "Home Sweet Home").

### 1.3 Start positions (Ch. 2 "Home Sweet Home", `StartPositions.png`/`lvl1.png` measured)

Actors start centered *on the boundary between two tile columns* (sprites are 16x16,
larger than a tile):

| Actor   | Start                                                        | Pixel center (measured) |
|---------|--------------------------------------------------------------|-------------------------|
| Pac-Man | between tiles (13,26) and (14,26)                            | ~(112, 212)             |
| Blinky  | outside, directly above the door: between (13,14) and (14,14)| ~(112, 116)             |
| Pinky   | house center slot: between (13,17) and (14,17)               | ~(112, 140)             |
| Inky    | house left slot: between (11,17) and (12,17)                 | ~(96, 140)              |
| Clyde   | house right slot: between (15,17) and (16,17)                | ~(128, 140)             |

"Blinky is always located just above and outside [the house], while the other three are
placed inside: Inky on the left, Pinky in the middle, and Clyde on the right." (Ch. 2
"Home Sweet Home"). These positions are restored after every life lost and level completed.

**Fruit spawn**: "the bonus symbols … appear directly below the monster pen" (Ch. 2 "The
Basics"). That is the center of the no-dot corridor at row 20, i.e. on the boundary of
tiles `(13,20)/(14,20)` (~pixel x=112, y=164). *The dossier gives no exact fruit
coordinates; the tile is inferred from "directly below the monster pen" plus the maze
geometry — same x as Pac-Man's start.*

### 1.4 Tile occupancy & movement model (Ch. 3 "What Tile Am I In?")

- An actor is associated with exactly **one tile**: the tile containing its **center
  point**, regardless of sprite overlap into neighboring tiles.
- Movement is **per-pixel**; occupancy updates the moment the center point crosses into
  the next tile.
- Boundary convention: in the pass-through-bug figure (Ch. 3 "Just Passing Through") the
  dossier states a center point "centered on the top edge of his tile … is still
  considered to be inside the bottom tile" — i.e. `tile = floor(center / 8)`, an edge
  pixel belongs to the tile it is the top/left edge of.
- All ghost pathfinding operates **only on tiles** ("it only cares about the tile an actor
  occupies—not its per-pixel location within that tile"), with each actor defined by
  (occupied tile, current direction). Distances between actors are measured in tiles.

---

## 2. Movement

### 2.1 Speed base and units

- **100% speed = 75.75757625 pixels/second** — stated in the Table A.1 header (HTML
  v1.0.27: "Table A.1 — Level Specifications (100% speed = 75.75757625 pixels/sec)").
  The older PDF print lacks this annotation.
- Frame rate is **60.606061 Hz** (Appendix C "Specifications": "Refresh rate —
  60.606061 Hz"). Therefore **100% = exactly 1.25 px/frame** (75.75757625 / 60.606061).
  All other speeds are percentages of this. (The task brief's guess of "1.46 px/frame"
  does not appear anywhere in the dossier.)
- The dossier expresses speeds only as percentages; it does not document the arcade's
  per-frame pixel-stepping patterns. For implementation, a fractional accumulator at
  `1.25 * pct` px/frame reproduces the documented rates.
- In-chapter speed table (Ch. 2 "Speed") — full per-level list → **Table A.1**:

  | Level | Pac norm | Pac norm, eating dots | Pac fright | Pac fright, dots | Ghost norm | Ghost fright | Ghost tunnel |
  |-------|----------|------|--------|------|------|------|--------|
  | 1     | 80%      | ~71% | 90%    | ~79% | 75%  | 50%  | 40%    |
  | 2–4   | 90%      | ~79% | 95%    | ~83% | 85%  | 55%  | 45%    |
  | 5–20  | 100%     | ~87% | 100%   | ~87% | 95%  | 60%  | 50%    |
  | 21+   | 90%      | ~79% | —      | —    | 95%  | —    | 50%    |

  The `~` values are the dossier's own approximations (the "dots" columns are effective
  averages produced by the 1-frame stop per dot, see §2.4) — the dossier prints them with
  `~` in both Ch. 2 and Table A.1; do not treat them as exact inputs. Implement the stop
  frames instead.
- Narrative version (Ch. 2 "Speed"): Pac-Man starts at 80% of maximum, reaches full speed
  by level 5, holds it through level 20, then drops to 90% for the rest of the game.
  Ghosts are slightly slower than Pac-Man until level 21, when they become faster than
  him. From level 21 on there is no frightened mode (see §4.4), hence the `—` entries.

### 2.2 Cornering (Ch. 2 "Cornering")

- Ghosts must reach the middle of a turn before changing direction; **Pac-Man does not**.
  He may begin a turn up to several pixels **before** the turn's center ("pre-turn") or
  **after** it ("post-turn").
- Exact pixel allowances (Ch. 2 "Cornering", `cornering_example2.png`): entering a turn
  tile **from the left: 3 pre-turn pixels** before the center point and **4 post-turn
  pixels** after it; entering **from the right: 4 pre-turn and 3 post-turn** pixels.
  Entering from the top vs. bottom "exhibits the same property" (same 3/4 vs 4/3
  asymmetry). With 8 pixels per tile this is 3 + center + 4 = 8, i.e. the turn center
  is the 4th pixel from the tile's left/top edge (0-based offset 3).
- During a pre/post-turn, Pac-Man's orientation changes immediately and he moves **one
  pixel in the new direction for every pixel in the old direction** (45° diagonal,
  effectively double speed) until he reaches the centerline of the new path, then resumes
  normal movement. Earliest possible pre-turn gains the most distance; every pixel of
  "lateness" costs one frame versus the optimum.

### 2.3 Buffered direction changes

The dossier describes no explicit input buffer beyond cornering: the player holds the
joystick in the intended direction "well before arriving at the center of a turn" and the
turn is taken at the earliest legal pre-turn pixel (Ch. 2 "Cornering"). Implementation
reading: the currently-held direction is continuously tested; it takes effect as soon as
it is legal (which, at a turn, is up to 3–4 px before the tile center). Reversal of
Pac-Man's own direction is allowed at any time (used by "head-faking", Ch. 4 Pinky).

### 2.4 Eating pauses (Ch. 2 "Speed")

- Eating a regular dot: Pac-Man "stops moving for one frame (1/60th of a second)" —
  ~10% slowdown, "just enough for a following ghost to overtake him".
- Eating an energizer: Pac-Man "stop[s] moving for three frames".

### 2.5 Tunnel (Ch. 2 "Speed", "Areas To Exploit"; Glossary "side tunnel")

- Any **ghost** entering the tunnel has "its speed … cut nearly in half" — exact tunnel
  percentages in the table above / → Table A.1. "This slow-down rule is always enforced
  and applies to ghosts only—Pac-Man is immune."
- Zone extent (measured from `exploit.png`, the pink zones): row 17, tiles cols **0–4**
  (left half) and cols **23–27** (right half) — "the two halves of the connecting
  side-tunnel" (Ch. 2 "Areas To Exploit").

---

## 3. Ghost behavior

### 3.1 The three modes (Ch. 2 "Modus Operandi")

Mutually exclusive: **chase** (hunt Pac-Man, per-ghost targeting, §3.6), **scatter**
(head for a fixed home-corner target for a few seconds), **frightened** (energizer eaten:
ghosts turn dark blue, wander randomly, are edible; on early levels only).

To a ghost, chase vs. scatter differ **only in where the target tile is** — identical
pathfinding either way (Ch. 3 "Target Tiles").

### 3.2 Scatter/chase schedule (Ch. 2 "Scatter, Chase, Repeat...")

Scatter happens **four times per level**, then chase continues indefinitely. The timer
resets when a life is lost or a level completes; ghosts leave the pen already in the
first scatter. Values in seconds:

| Phase     | Level 1    | Levels 2–4 | Levels 5+  |
|-----------|------------|------------|------------|
| Scatter 1 | 7          | 7          | 5          |
| Chase 1   | 20         | 20         | 20         |
| Scatter 2 | 7          | 7          | 5          |
| Chase 2   | 20         | 20         | 20         |
| Scatter 3 | 5          | 5          | 5          |
| Chase 3   | 20         | **1033**   | **1037**   |
| Scatter 4 | 5          | **1/60**   | **1/60**   |
| Chase 4   | indefinite | indefinite | indefinite |

- The 1/60-second scatter 4 "appears as a simple reversal of direction by the ghosts".
- **Frightened mode pauses the scatter/chase timer**; when frightened time runs out the
  ghosts "return to the mode they were in before being frightened and the scatter/chase
  timer resumes".
- (Cross-check: identical in PDF and HTML.)

### 3.3 Reversal rule (Ch. 2 "Reversal Of Fortune")

- Ghosts are prohibited from reversing direction by their own choice, in **all** modes.
- The system forces a reversal on these mode transitions: **chase→scatter,
  chase→frightened, scatter→chase, scatter→frightened**. "Ghosts do not reverse direction
  when changing back from frightened to chase or scatter modes."
- The reversal is not necessarily simultaneous: each ghost obeys the pending reversal
  signal **when it next enters a new tile** after the signal.

### 3.4 Pathfinding: look-ahead + intersections (Ch. 3 "Looking Ahead", "Intersections")

Algorithm (shared by all four ghosts):

1. On entering a new tile, the ghost looks **one tile ahead** along its current direction
   of travel and decides *now* which way it will go **when it reaches that tile**. On
   arriving there it applies the stored decision, then repeats.
2. For the look-ahead tile, consider the four exits (up/down/left/right). Discard exits
   blocked by walls, and discard the exit that reverses travel (ghosts never voluntarily
   reverse).
3. If one exit remains, take it. If several remain, collect a **"test tile"** one tile
   beyond the look-ahead tile in each remaining direction, and compute the **straight-line
   (Euclidean) distance** from each test tile to the target tile. Choose the direction
   whose test tile is nearest the target. (The dossier says the ghost "triangulates the
   distance"; Ch. 4 Clyde names it "the Euclidean distance". Note: comparing squared
   distances is equivalent.)
4. **Tie-break** on equal distances by direction preference: **up > left > down > right**
   ("Up is the most preferred direction; right is the least").
5. Ghosts cannot see far ahead; no path search of any kind — this greedy one-tile
   decision is the whole algorithm.

### 3.5 Fixed targets: scatter corners & eyes (Ch. 3 "Fixed Target Tiles")

Each scatter target is "in dead space above or below the actual maze making them
impossible for the ghosts to reach" — that is all scatter mode is: chasing an unreachable
corner tile, which makes each ghost loop around its corner. Measured pixel-precisely from
the dossier's `Scatter.png` diagram (see Ambiguities §8.1 for calibration detail):

| Ghost  | Scatter target (col,row) | Where                                   |
|--------|--------------------------|-----------------------------------------|
| Pinky  | **(2, −1)**              | above the screen, near top-left          |
| Blinky | **(25, −1)**             | above the screen, near top-right         |
| Clyde  | **(0, 34)**              | bottom-left, one row below the maze wall |
| Inky   | **(27, 34)**             | bottom-right, one row below the maze wall|

(Equivalently: top targets are 4 rows above the maze's top wall row at cols 2 / 25;
bottom targets are 1 row below the maze's bottom wall row at cols 0 / 27. Many secondary
sources quote these one row lower — `(2,0) (25,0) (0,35) (27,35)`; see §8.1. The
behavioral difference is negligible since only relative distances at decision points
matter, but the diagram measurement is as given above.)

- **Eyes target**: an additional fixed target used by eaten ghosts' eyes, "located
  directly above the left side of the 'door'": tile **(13, 14)** (text + measured green
  tile in `Scatter.png`; consistent with door at (13,15)/(14,15)).

### 3.6 Chase targeting per ghost (Ch. 4 "Meet The Ghosts")

- **Blinky** ("Blinky"): target = **Pac-Man's current tile**. Simplest and most direct.
- **Pinky** ("Pinky"): target = **4 tiles ahead of Pac-Man** in Pac-Man's current
  direction of travel — except **when Pac-Man faces up, the target is 4 tiles up AND
  4 tiles to the left** of Pac-Man's tile. This is the overflow bug, stated exactly:
  "if Pac-Man is moving up, Pinky's target tile will be four tiles up and four tiles to
  the left … due to a subtle error in the logic code … an overflow bug that mistakenly
  includes a left offset equal in distance to the expected up offset."
- **Inky** ("Inky"): two-step calculation. (1) Compute an intermediate offset tile
  **2 tiles ahead of Pac-Man** in his direction of travel — with the same up-bug: facing
  up gives **2 up AND 2 left**. (2) Draw the vector from **Blinky's current tile** to
  that offset tile and **double it**; the tile the doubled vector lands on is Inky's
  target. (I.e. `target = offset + (offset − blinky) = 2*offset − blinky`, componentwise
  in tiles.)
- **Clyde** ("Clyde"): compute the **Euclidean distance** from Clyde's tile to Pac-Man's
  tile. If the distance is **≥ 8 tiles**, target Pac-Man's tile exactly like Blinky; if
  **< 8 tiles**, target switches to Clyde's **scatter target** (bottom-left corner). This
  makes Clyde orbit near Pac-Man at close range but he is "still dangerous if you manage
  to get in his way".
- Head-faking (Glossary, Ch. 4 Pinky/Inky): Pinky and Inky use Pac-Man's *direction*, so
  reversing direction toward them can redirect them; "Blinky and Clyde do not use
  Pac-Man's current direction in their chase logic, so they are unaffected".
- Regardless of where a ghost's target is, "Pac-Man will still be killed if he gets in
  that ghost's way" (Ch. 4 intro) — collision does not depend on targeting.

### 3.7 Red zones — no upward turns (Ch. 2 "Areas To Exploit")

- Two zones where "ghosts are forbidden to make upward turns"; inside them a ghost "may
  only travel from right-to-left or left-to-right until exiting the area".
- Measured from `exploit.png` (red bars): **row 14, cols 11–16** (corridor above the
  house) and **row 26, cols 11–16** (Pac-Man's start corridor). Within those corridor
  segments the only upward exits are at cols 12 and 15, so the effective rule is: at
  decision tiles **(12,14), (15,14), (12,26), (15,26)** the UP exit is excluded from a
  ghost's choices. (This is why only Pac-Man can enter those four upward-facing passages
  from below; ghosts can still enter them from the top end.)
- "The red zone restrictions are enforced during both scatter and chase modes, but in
  frightened mode the red zones are ignored temporarily, allowing the ghosts to turn
  upwards if they so choose."
- *Eaten eyes:* the dossier does not say whether eyes honor the red zones (see §8.4).

### 3.8 Frightened mode (Ch. 2 "Frightening Behavior"; "Modus Operandi"; "The Basics")

- Trigger: Pac-Man eats an energizer → all ghosts reverse (always, on every level) and,
  on levels that have frightened time, turn dark blue and become edible.
- Frightened duration and number of white flashes are per-level → **Table A.1** (level 1:
  6 seconds, 5 flashes). "As the levels progress, the time ghosts spend in frightened
  mode grows shorter until eventually they no longer turn blue at all (they still reverse
  direction, however)." — "By level 19, the ghosts stop turning blue altogether and can
  no longer be eaten" (Ch. 2 "The Basics"). Ghosts "flash white briefly as a warning"
  before reverting.
- Frightened speed: much slower (50/55/60% by level band, → Table A.1); Pac-Man speeds up
  while ghosts are frightened on levels 1–4 (90/95%).
- **Random turning**: frightened ghosts pick turns via PRNG. Exact mechanism (Ch. 2
  "Frightening Behavior"): "The PRNG generates an pseudo-random memory address to read
  the last few bits from. These bits are translated into the direction a frightened ghost
  must first try. If the selected direction is not blocked by a wall or opposite the
  ghost's current direction of travel, it is accepted. Otherwise, the code proceeds in a
  **clockwise** fashion to the next possible direction and tries again," until an
  acceptable direction is found. The PRNG "gets reset with the same initial seed value at
  the start of each new level and whenever a life is lost" (this is what makes patterns
  deterministic). Implementation: any PRNG giving a direction in {up,right,down,left};
  on rejection rotate clockwise (up→right→down→left→up). Reseed on level start and death.
- Frightened decisions replace targeting entirely (no target tile); the reverse-direction
  prohibition still applies (see quote above), and red zones are ignored (§3.7).
- **Eating ghosts**: first ghost after an energizer = **200**, then **400, 800, 1600**
  for successive ghosts *from the same energizer* (resets to 200 at each new energizer).
  All four ghosts at all four energizers = extra 12,000/level (Ch. 2 "The Basics").
- After being eaten, "a ghost's eyes will return to the monster pen where it is
  resurrected, exiting to chase Pac-Man once again" (Ch. 2 "The Basics"). Eyes head for
  the fixed tile above the door (§3.5), enter the house through the door, are revived,
  and exit; a revived Blinky "immediately turns around to leave once revived" (Ch. 2
  "Home Sweet Home"). *Eyes speed, the score-popup pause when a ghost is eaten, and the
  exact door-transit path are not specified by the dossier — see §8.4.*

### 3.9 Ghost house: movement, exit and entry (Ch. 2 "Home Sweet Home"; Ch. 4 Blinky)

- Ghosts waiting inside the house **bounce up and down** in their slots (Ch. 4 Blinky:
  "…until the orange ghost (Clyde) stops bouncing up and down inside the ghost house and
  moves toward the door to exit"). Exit = stop bouncing, move toward the door, out
  through it. *(The dossier gives no more precise exit path; the standard reading is:
  align horizontally to the house center x=112, rise through the door to Blinky's start
  point, then begin normal play — inference, see §8.4. House/door speeds are not given.)*
- **Exit direction**: "Ghosts typically move to the left once they get outside, but if
  the system changes modes one or more times when a ghost is inside, that ghost will move
  to the right instead of the left upon leaving the house."
- Once out, a ghost cannot re-enter except as eaten eyes.

### 3.10 Leaving the house: dot counters (Ch. 2 "Home Sweet Home")

Applies to Pinky, Inky, Clyde only (Blinky is never subject to it).

**Personal dot counters** (normal play):

- Each of the three has a dot counter, reset to zero at level start, active only while
  that ghost is in the house, and **only one counter is active at a time**, preference
  order **Pinky, then Inky, then Clyde**. Every dot Pac-Man eats increments the active
  (most-preferred housed) ghost's counter. When a ghost's counter reaches/exceeds its
  **dot limit**, it exits immediately and its counter deactivates (not reset); the next
  preferred housed ghost's counter activates.
- Dot limits: **Pinky always 0** (leaves immediately every level). **Level 1: Inky 30,
  Clyde 60. Level 2: Inky 0, Clyde 50. Level 3+: all 0** — everyone leaves immediately.

**Global dot counter** (after a death):

- Losing a life **disables (does not reset) the personal counters** and enables a
  **global** counter, reset to 0, counting dots from that point. Releases: **Pinky at 7,
  Inky at 17**; and when it reaches **32**: *if Clyde is inside the house at that
  moment*, the global counter is **reset to zero and deactivated** and the personal
  limits are used again (including for Clyde). If Clyde is *not* in the house when the
  count hits 32, the counter can never be deactivated and keeps counting — and since it
  only checks 7/17/32, ghosts eaten later and sent home can then only be released by the
  no-dot timer. (This is the documented "keep the ghosts in the house" trick/bug.)

**No-dot-eaten timer** (always running):

- A timer tracks time since the last dot was eaten (reset on every dot). When it hits its
  limit, "the most-preferred ghost waiting in the ghost house (if any) is forced to leave
  immediately" and the timer resets. Limit: **4 seconds on levels 1–4, 3 seconds from
  level 5** ("The game begins with an initial timer limit of four seconds, but lowers to
  it to three seconds starting with level five.").

### 3.11 Cruise Elroy (Ch. 4 "Blinky"; Glossary "Cruise Elroy")

- Blinky speeds up **twice per level** based on dots remaining. Level 1: at **20 dots
  left** he becomes "Elroy 1", "accelerating to be at least as fast as Pac-Man"; at
  **10 dots left**, "Elroy 2", "moving faster than Pac-Man". Thresholds rise with level;
  dot counts and both Elroy speeds per level → **Table A.1** (level 1: Elroy1 20 dots /
  80%, Elroy2 10 dots / 85%).
- While Elroy, Blinky's **scatter behavior changes**: he keeps targeting Pac-Man's
  current tile instead of his corner for all remaining scatter periods of the level —
  but he **still reverses direction** on scatter entry/exit like everyone else.
- **After a life is lost**, Elroy is temporarily suspended: "Blinky's 'Cruise Elroy'
  abilities are temporarily suspended until the orange ghost (Clyde) stops bouncing up
  and down inside the ghost house and moves toward the door to exit. Until this happens,
  Blinky's speed and scatter behavior will remain normal regardless of the number of dots
  remaining in the maze." Then Elroy resumes based on dot count. (The dossier ties this
  suspension to losing a life; it does not mention it at ordinary level start, where
  Elroy is moot anyway because all dots are present.)

---

## 4. Collisions & death

### 4.1 Collision rule (Ch. 3 "Just Passing Through")

- **Collision = Pac-Man and a ghost occupy the same tile** ("Any time Pac-Man occupies
  the same tile as a ghost, he is considered to have collided with that ghost and a life
  is lost."). It is irrelevant who moved into whose tile. Not pixel-based.
- **Pass-through bug** (faithful behavior): if, in the same 1/60 s frame, Pac-Man's
  center crosses into the ghost's tile while the ghost's center crosses into Pac-Man's
  tile, they **swap tiles without ever sharing one** and no collision occurs. Rare
  (">99%" of the time the tile test suffices) but real; keep tile-based per-frame checks
  to preserve it.
- The same tile test presumably governs eating a frightened ghost (the dossier describes
  the collision test once, in the death context; it does not restate it for frightened
  ghosts).

### 4.2 Death & reset (Ch. 2 "The Basics", "Home Sweet Home", "Scatter, Chase, Repeat...")

On capture: "a life is lost, the ghosts are returned to their pen, and a new Pac-Man is
placed at the starting position before play continues." Specifically:

- All actors return to their §1.3 start positions (Blinky outside above the door).
- **Dots/energizers are preserved** — only eating all 244 completes a level; and Ch. 5
  notes the nine right-half dots of level 256 respawn on death *as an anomaly*, implying
  normal dots never respawn.
- The **scatter/chase timer resets** (ghosts emerge in the first scatter period).
- Personal dot counters are disabled; the **global dot counter** activates at 0 (§3.10).
- The frightened **PRNG is reseeded** (§3.8). Cruise Elroy is suspended until Clyde moves
  to exit (§3.11).
- Game over when captured with no extra lives remaining (Ch. 2 "The Basics").
- *The dossier does not document the death animation timing or freeze frames — §8.4.*

---

## 5. Pellets, fruit, scoring

### 5.1 Scoring summary (Ch. 2 "The Basics"; Glossary)

| Item                                | Points                                  |
|-------------------------------------|-----------------------------------------|
| Small dot (240 of them)             | 10                                      |
| Energizer (4)                       | 50                                      |
| Ghosts, per energizer chain         | 200 / 400 / 800 / 1600                  |
| Bonus fruit, by level               | 100 … 5000 → **Table A.1** (L1 cherry 100; glossary: strawberry 300, peach 500, grapes 1000, galaxian 2000, key 5000) |
| Clear all 244 dots                  | 2600 total from dots; level ends        |

- Score display rolls over at 999,999 ("flipping", Glossary); internal score keeps
  counting.

### 5.2 Fruit (Ch. 2 "The Basics")

- Appears **twice per level**, "directly below the monster pen" (position §1.3): first
  when **70 dots** have been cleared, second at **170 dots**.
- Duration: "always between nine and ten seconds. The exact duration (i.e., 9.3333
  seconds, 10.0 seconds, 9.75 seconds, etc.) is variable and does not become predictable
  with the use of patterns." → implement as a uniformly random 9.0–10.0 s timer.
- The fruit/level counter along the bottom edge shows the symbols of the last six rounds
  plus the current round.
- *Score popup on eating fruit (value shown at spawn point) is visible in dossier
  screenshots but its duration is not documented — §8.4.*

### 5.3 Extra life

- **Not documented in the dossier** (neither the text nor Table A.2 covers the bonus-life
  DIP switch). Implement the machine default of one extra life at **10,000 points** as
  standard arcade default; source that value elsewhere (e.g. the operator's manual scans
  referenced in Appendix C). Flagged in §8.4.
- Starting lives: also not stated in the dossier text ("three lives" is the arcade
  default DIP; the FAQ's perfect-score description mentions "five extra men" as the
  best-case DIP configuration). Flagged in §8.4.

### 5.4 Level completion (Ch. 2 "The Basics"; Ch. 5 "Playing The Level")

- Level is complete when all **244** dots are eaten: "The game does not consider a level
  to be completed until 244 dots have been eaten" (Ch. 5). Then "the board is reset, and
  a new round begins" (Ch. 2 "The Basics").
- The scatter/chase timer resets on level completion; ghosts return to start positions.
- *End-of-level white/blue maze flash animation: not documented in the dossier (count or
  timing) — §8.4.*

---

## 6. Timing & misc

- **Frame rate: 60.606061 Hz** (Appendix C). The dossier's prose says "1/60th of a
  second" for one frame informally; use frames as the timing unit and treat all "seconds"
  values as seconds at 60.606061 fps (or, pragmatically, 60 fps — the dossier itself
  conflates the two; flagged §8.3).
- All timers to implement in frames: scatter/chase phases (§3.2), frightened time and
  flashes (→ Table A.1), no-dot timer 4 s / 3 s (§3.10), fruit 9–10 s (§5.2), dot stop
  1 frame / energizer stop 3 frames (§2.4).
- **READY! sequence / start jingle**: not documented in the dossier (neither duration nor
  structure). Known from the game itself: "PLAYER ONE / READY!" with jingle on first life,
  shorter "READY!" on subsequent lives — durations must be sourced elsewhere or tuned by
  ear. Flagged §8.4.
- **Initial directions**: not stated in the dossier for Pac-Man or Blinky at level start.
  The house-exit rule (§3.9: exit moving left, or right if a mode change happened while
  inside) covers Pinky/Inky/Clyde. Flagged §8.4.
- **Attract mode**: not needed; only dossier note of interest: in the attract demo,
  Pac-Man dies to Inky on normal difficulty and to Clyde on hard (Table A.2 preamble).
- **Level 256** (Ch. 5): the level counter is a byte; on level 256 (internal counter 255,
  zero-based) the fruit-drawing routine increments 255→0 and then draws 256 symbols,
  trashing the right half of the screen. Left half plays normally; only 168 of 244 dots
  are obtainable (114 visible left + 9 right, respawning per death), so the level cannot
  be completed. For `pacmantui`: use a wider level counter and simply continue past 255
  (safe continuation), optionally noting the historical bug.
- **Hard difficulty** (Table A.2 preamble): a PCB jumper removes levels 1, 3, 6, 19, 20
  from the sequence (bonus symbols/values keep their normal per-board identity). Not
  needed for a default implementation; details → Table A.2.

---

## 7. Quick implementation checklist (all rules above)

1. 28x36 grid, 8 px tiles; playfield rows 3–33; maze per §1.2; wrap on row 17.
2. Actors: pixel positions + derived tile (floor/8); start per §1.3.
3. Speeds as % of 1.25 px/frame; per-level table → Table A.1; dot/energizer stop frames.
4. Pac-Man cornering with 3/4-px pre/post-turn window and 45° cut.
5. Mode state machine per §3.2 with forced reversals (§3.3) applied on next tile entry;
   frightened pauses the phase timer.
6. Ghost decision engine: look-ahead one tile, Euclidean test tiles, up>left>down>right
   tie-break, no voluntary reversal, red-zone up-exclusion at the four tiles (§3.7).
7. Targets: scatter corners & eyes tile (§3.5), chase rules incl. Pinky/Inky up-bug
   (§3.6), Clyde's 8-tile switch, Elroy overrides (§3.11).
8. Frightened: clockwise-retry random turns, reseeded PRNG per level/death.
9. House release: personal counters (0/30/60 → 0/0/50 → all 0), global 7/17/32 with
   Clyde-deactivation rule, no-dot timer 4 s→3 s.
10. Tile-based collision each frame (pass-through preserved), death reset per §4.2.
11. Dots 10, energizers 50, ghost chain 200–1600, fruit at 70/170 dots for rand(9–10) s.
12. Level complete at 244; counters/timers reset; layout identical every level.

---

## 8. Ambiguities & resolutions

### 8.1 Scatter target rows (one-row discrepancy)

The dossier's only source for scatter-target positions is the `Scatter.png` diagram
(Ch. 3 "Fixed Target Tiles"); the text just says the targets are "in dead space above or
below the actual maze". The diagram image is 37 tile-rows tall (one extra row added above
the screen area — verified: its maze walls and ghost-house door sit exactly one tile
lower than in `Tiles.png`, whose 36-row geometry matches the real screen). Measuring the
solid target squares against that calibration gives Pinky (2,−1), Blinky (25,−1),
Clyde (0,34), Inky (27,34) in screen coordinates — top targets one row *above* the
visible screen, bottom targets on the bottom HUD row. The calibration is independently
confirmed by the same image's green eyes-target square landing on (13,14), which matches
the text "directly above the left side of the door" (door measured at (13,15)/(14,15)).
Secondary sources commonly quote (2,0), (25,0), (0,35), (27,35) — exactly what one gets
by misreading the 37-row diagram as 36 rows. **Resolution:** use the measured values;
the behavioral difference against the one-row-lower variant is negligible (targets are
unreachable; only relative distances at decision tiles matter), so either choice yields
essentially the classic behavior, but the diagram measurement is the dossier's own data.

### 8.2 Values the dossier itself marks approximate

- The "dots" speed columns (~71%, ~79%, ~83%, ~87%) carry `~` in the dossier — they are
  observed averages caused by the 1-frame stop per dot, not engine inputs. Implement the
  stop frames, not the percentages.
- Fruit duration is explicitly variable, "always between nine and ten seconds", not
  pattern-predictable (Ch. 2 "The Basics"). Random 9–10 s is faithful.
- Ghost tunnel slowdown is introduced as "cut nearly in half" in prose; exact percentages
  are in the speed table / Table A.1 (40/45/50%).

### 8.3 1/60 s vs 60.606061 Hz

The dossier states the hardware refresh as 60.606061 Hz (Appendix C) yet describes one
frame as "1/60th of a second" in prose and expresses the scatter-4 phase as "1/60th of a
second" (= 1 frame). **Resolution:** treat every "second" as a count of frames at the
real 60.606061 fps for authenticity (1 s ≈ 60.6 frames); using flat 60 fps changes phase
lengths by ~1%, which the dossier's own prose already rounds over. Scatter 4 on levels
2+ is exactly **1 frame**.

### 8.4 Mechanics the dossier does not document (need another source or tuning)

- **Ghost-eaten pause** (the ~1 s freeze while the 200/400/800/1600 score shows and only
  eyes move): not mentioned anywhere in the dossier.
- **Eyes speed** (eyes are visibly faster than ghosts) and door-transit behavior for
  eyes/house exits (alignment path, speed inside house and through door): not specified.
  The bounce, the "moves toward the door" exit and the door tiles are documented; the
  rest is inference (§3.9).
- Whether **eaten eyes honor the red zones**: unstated. (Only "enforced during both
  scatter and chase modes, but in frightened mode … ignored" is given.) Practical
  resolution: exempt eyes so they can path home efficiently; revisit if a primary source
  is found.
- **Death sequence timing/animation**, **end-of-level flash count/timing**, **READY!**
  timings, **start jingle length**: absent from the dossier.
- **Starting lives** and **extra life at 10,000**: DIP-switch matters not covered by the
  dossier text or its Table A.2 (which is solely the hard-difficulty level mapping).
  Use arcade defaults (3 lives; bonus life at 10,000).
- **Initial facing directions** of Pac-Man and Blinky at level start: unstated (both face
  left in the arcade game; the dossier's screenshots are consistent with that but it
  never says so).
- **Exact fruit tile**: "directly below the monster pen" only; position in §1.3 is
  inferred from geometry (x = Pac-Man start x, row 20).
- **Frightened-ghost eating collision**: the tile-collision rule is stated for death;
  the dossier does not restate it for eating a blue ghost (assume same tile test).

### 8.5 Source-version notes

- The PDF print (v1.0.26, Jan 2015) is missing printed page 12 (part of Ch. 2 between
  "Reversal Of Fortune" and the mode table) and omits the "100% speed = 75.75757625
  pixels/sec" annotation on Table A.1. The HTML mirror (v1.0.27) contains both; all
  content here was verified against the HTML.
- Everything in the dossier is claimed to be "extracted from or verified with disassembly
  output from the original Pac-Man code ROMs" (Introduction), so where it speaks
  precisely it can be trusted as ground truth.
