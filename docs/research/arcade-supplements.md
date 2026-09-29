# Arcade Supplements — values the Pac-Man Dossier does not document

Companion to `dossier-mechanics.md`. Resolves every open item in its §8 ("Ambiguities &
resolutions"), with disassembly-grade citations. Research date: 2026-09-28.

## 0. Sources & method

Primary (disassembly-grade):

- **S1 — Original Pac-Man commented disassembly** (Midway ROM set, full Z80 opcode listing
  with RAM annotations): <https://cubeman.org/arcade-source/pacman.asm>. All ROM addresses
  and byte values below were read directly from this listing.
- **S2 — Scott Lawrence (umlautllama/"BleuLlama") Ms. Pac-Man documented disassembly**
  (`mspac.asm`, contributors incl. Don Hodges, Dave Widel, Mark Spaeth):
  <https://github.com/BleuLlama/GameDocs/blob/master/disassemble/mspac.asm>.
  Ms. Pac-Man is a daughterboard patch on the original Pac-Man program: addresses
  0x0000–0x3FFF are the original Pac-Man code except where GCC patched it. S2's extensive
  comments were used to *interpret* routines; **every timer byte, constant and instruction
  cited below was then verified byte-for-byte against the original ROM in S1** at the same
  address (verified regions: 0x0219–0x0262, 0x0664–0x06c2 state table, 0x0879–0x0a2c,
  0x0c42–0x0d93, 0x0ead–0x0f1c, 0x10c0–0x1235, 0x1235–0x1375, 0x1717–0x1806,
  0x1b36–0x1c4a, 0x2069–0x20d7, 0x253d–0x2728, 0x2730–0x28e2). Where Ms. Pac-Man
  diverges (fruit movement, randomized scatter, cut-scenes) only the S1 original bytes
  were used.
- **S3 — MAME driver** `src/mame/pacman/pacman.cpp`:
  <https://github.com/mamedev/mame/blob/master/src/mame/pacman/pacman.cpp> (DIP defaults).

Secondary (corroboration only):

- **S4 — The Pac-Man Dossier**, Jamey Pittman —
  <https://www.gamedeveloper.com/design/the-pac-man-dossier> (HTML v1.0.27 mirror at
  pacman.holenet.info). Already fully extracted in `dossier-mechanics.md`.
- **S5 — Shaun Williams' arcade-accurate remake** (built from the dossier plus
  correspondence with Pittman): <https://github.com/shaunlebron/pacman>
  (mirror: <https://github.com/masonicGIT/pacman>) — `src/Ghost.js`, `src/Actor.js`.
- **S6 — "Understanding Pac-Man Ghost Behavior"**, Chad Birch. The live domain
  (gameinternals.com) currently serves spam; use the archive:
  <https://web.archive.org/web/2020*/gameinternals.com/understanding-pac-man-ghost-behavior>.

### 0.1 Frame & timer conventions (needed to read every value below)

One frame = one 60.606061 Hz VBLANK interrupt (dossier App. C). The game's own clock
chain (counters `4C86–4C89`, update routine at `0x01DC`, limits table at `0x0219`:
`06 A0 0A 60 0A 60 0A A0`, S1/S2) treats **60 frames = 1 second**, so every "second"
below is exactly 60 frames ≈ 0.99 s wall-clock.

**Timed tasks** (`rst #30`, 3 inline bytes: timer, task#, param; dispatcher at `0x0221`):
the timer byte's top 2 bits select the tick unit, the low 6 bits the tick count
(S2 RAM-map: "0x40 → 10 hundredths, 0x80 → 1 second, 0xC0 → 10 seconds"; unit 0x00 =
every frame). Decisive detail: tasks tick on **global clock boundaries** (every 6 frames
for unit 0x40, every 60 frames for unit 0x80), *not* relative to insertion time. A task
with count N therefore fires after `6·(N−1) + (1..6)` frames (unit 0x40) or
`60·(N−1) + (1..60)` frames (unit 0x80), depending on clock phase at insertion. This
phase jitter is the ROM's *mechanism* for several "variable" durations below. Reference
windows used throughout:

| timer byte | unit | count | frames (window) | ≈ seconds |
|-----------|------|-------|-----------------|-----------|
| `0x42` | 0.1 s | 2 | 7–12 | 0.12–0.20 |
| `0x43` | 0.1 s | 3 | 13–18 | 0.21–0.30 |
| `0x4A` | 0.1 s | 10 | 55–60 | 0.91–0.99 |
| `0x54` | 0.1 s | 20 | 115–120 | 1.90–1.98 |
| `0x57` | 0.1 s | 23 | 133–138 | 2.19–2.28 |
| `0x8A` | 1 s | 10 | 541–600 | 8.93–9.90 |

The core gameplay routine `#1017` is called **twice per frame** (`0x08EB`/`0x08EE`, S1) —
this doubles actor-update opportunities (speed patterns, eyes) and makes the death
counter (incremented inside `#1017`) count **half-frames**.

Speed reference: dossier "100%" = 75.7576 px/s = **1.25 px/frame**.

---

## 1. Ghost-eaten pause

**Resolved: 1.0 second (55–60 frames; 60 in the common phase-locked case).**
ROM-derived (S1/S2, byte-identical).

Mechanism (`#1235`–`#1290`, reached from `#102E` while `4DA4` ≠ 0):

- Collision with a blue ghost (`#171D`/`#1789`) sets `4DA4` := ghost index. On the next
  `#1017` iteration the pause handler runs once: the eaten ghost's sprite slot is
  replaced by the **score sprite** (`sprite := 4DD0 + 0x27` → sprites 0x28/0x29/0x2A/0x2B
  = 200/400/800/1600, color 0x18), **Pac-Man is hidden** by setting his color to 0
  (`0x1269: ld (#4c0b),#00`), and timed task `4A 03 00` (= 10 × 0.1 s ticks ≈ **1.0 s**)
  is scheduled (`0x126E`).
- While `4DA4` ≠ 0, `#1017` returns right after the dead-ghost handlers (`#102E`), so:
  **frozen** — Pac-Man (hidden), all live ghosts' positions, collision checks, the
  frightened-mode timer (`#1376` not reached), and in-house bouncing (`#0C42` gates on
  `4DA4` at `0x0C42`). **Still running** — *eyes of previously eaten ghosts keep moving*
  (handlers `#1094/#109E/#10A8/#10B4` run before the gate), ghost sprite animation
  (`#0E23`, called outside `#1017`), frightened blue/white color flashing (`#0AC3`),
  energizer blinking (`#0C0D`), sound, and all IRQ timed tasks (fruit despawn timer
  included).
- When the task fires, the score sprite becomes the **eyes** sprite (0x20), Pac-Man's
  color is restored to 9, the ghost's state flips to "dead", and the eyes-returning
  sound starts (`0x1277–0x1290`).

Implementation note: the frightened timer *pauses* during this second; the
scatter/chase timer also does not advance (it is driven from the same gated path).

Confidence: **ROM-derived.**

## 2. Eyes (eaten ghost) return speed

**Resolved: exactly 2 pixels per frame, unconditionally = 160% of "100%" speed.**
ROM-derived.

Evidence: a live ghost's movement is gated by its 32-bit speed pattern (e.g. red:
`#1B36`–`#1BD7`, S1/S2) before falling into the 1-px move routine `#1BD8`. A **dead**
ghost's per-state handler (`#10C0` for red, etc.) calls `#1BD8` **directly**, skipping
the speed-pattern gate *and* the tunnel-slowdown check — and these handlers run in
`#1017`, i.e. **twice per frame** → 2 px moved every frame, everywhere (tunnel included),
including during the ghost-eaten pause. At the dossier's 1.25 px/frame = 100% scale,
eyes travel at **160%**.

Corroboration: S5 `Ghost.prototype.getNumSteps`: `if (mode == GHOST_GOING_HOME ||
mode == GHOST_ENTERING_HOME) return 2;` (2 px/frame).

Sources that say only "faster than normal" are superseded; no qualitative fallback
needed.

Confidence: **ROM-derived.**

## 3. Death sequence timings

**Resolved (all ROM-derived).** Driven by counter `4DC5` (zeroed by the task-#11 RAM
clear at life start), incremented **twice per frame** inside `#1291`; thresholds at
`0x12BE`–`0x1365` (S1 bytes identical to S2: `0x78, 0xB4, 0xC3, 0xD2, 0xE1, 0xF0, 0xFF,
0x10E, 0x11D, 0x12C, 0x13B, 0x159, 0x1B8`). Divide counts by 2 for frames:

| phase | counter | frames after collision | duration |
|---|---|---|---|
| Freeze — everything stops in place, all actors visible | 0 → 0x78 | 0 → 60 | **60 f (≈1.0 s)** |
| Ghosts hidden (`#267E` zeroes `4D00–4D07`); Pac-Man shows pre-death sprite 0x34 | 0x78 → 0xB4 | 60 → 90 | 30 f |
| Death animation proper: sprites 0x35–0x3D, **7.5 frames each**; dying sound starts at its first frame | 0xB4 → 0x13B | 90 → 157.5 | 67.5 f |
| Final sprite 0x3E ("pop"); sound stopped | 0x13B → 0x159 | 157.5 → 172.5 | 15 f |
| Blank (sprite 0x3F), screen empty of actors | 0x159 → 0x1B8 | 172.5 → 220 | 47.5 f |
| Lives decremented, state machine advances | 0x1B8 | 220 | — |

- **Collision → animation start: 90 frames** (60 freeze + 30 pre-death sprite).
  Animation (0x35–0x3E incl. sound): **82.5 frames**. Post-animation blank: **47.5 f**.
  **Collision → end of death state: 220 frames ≈ 3.63 s.**
- Then (lives remaining) states `4E04` 4→5→6→9 advance in ~3 frames and the READY!
  respawn phase runs (**item 4**: 115–120 frames). **Collision → control returned ≈
  338–343 frames ≈ 5.6 s.**
- What other actors do: during the 60-frame freeze the outside ghosts stop moving but
  their sprite animation *continues* (`#0E23` runs outside `#1017`), energizers keep
  blinking, and **ghosts inside the house keep bouncing** (`#0C42` is not gated on
  `4DA5`). At frame 60 all four ghosts disappear at once; Pac-Man alone plays the
  animation. A fruit on screen stays (its IRQ despawn timer keeps running and may
  expire mid-sequence).
- Game over instead: state 6 draws "GAME OVER" (+ credits) and holds on timer `0x54`
  (115–120 f) before returning to attract/2P-swap (`0x0952`–`0x0960`).

Confidence: **ROM-derived.** (The 7.5-frame animation step is exact — thresholds are
odd counts of a 2-per-frame counter.)

## 4. READY! sequences

**Resolved (ROM-derived).** All handled by main-state `4E04` machine (`#06BE` jump
table) and timed tasks.

**First game start (after pressing START, `#0674`):** two phases.

- *Phase A — "PLAYER ONE" + "READY!", no actors*: `#0674` draws maze, pellets,
  "PLAYER ONE" (text 3), "READY!" (text 6), scores, fruit stock, lives; the intro
  jingle starts with the credit/start handling. Duration: timed task `57 01 00` at
  `0x069F` = 23 × 0.1 s ticks = **133–138 frames ≈ 2.2 s**.
- *Phase B — actors appear, "READY!" only*: state 1 (`#0899`) clears actor RAM
  (task #11), **clears "PLAYER ONE"** (task #1C param 0x83), resets actors to their
  start positions/directions (task #04 → `#253D`, see item 7), resets the house-speed
  counter (task #05), then holds on two `54` tasks (`0x08B7`, `0x08BB`): advance to
  gameplay and erase "READY!" after **115–120 frames ≈ 1.95 s**.
- Total hold ≈ **248–258 frames ≈ 4.1–4.3 s**, which is what the ~4.2 s intro jingle
  spans. (Jingle audio length itself not extracted from the sound tables —
  **documented approximation**; the two state timers above are the exact gameplay
  gate.)

**Subsequent lives and every new level:** single phase, state 9 (`#0988`, also reached
via `#0AA0 → jp #0988` between levels): maze + remaining/full pellets redrawn, actors
reset (tasks #11/#13/#04/#05), "READY!" drawn, hold on `54 00 00` + `54 06 00` =
**115–120 frames ≈ 1.95 s**, then `4E04 := 3` (play) via `#09D2`. Actors are visible at
their spawn points for the whole phase; READY! is erased at the same moment control
begins.

Confidence: **ROM-derived** (both `0x57` and `0x54` bytes verified in S1 at
`0x06A0`, `0x08B8/0x08BC`, `0x09B7/0x09C1`).

## 5. Level-complete sequence

**Resolved (ROM-derived).** States `0x0C`–`0x25` of `4E04` (handlers `#09D8`–`#0AA3`,
S1 bytes identical to S2):

1. 244th dot eaten → `4E04 := 0x0C` (`0x08E5`); next frame `#09D8` silences sound and
   schedules `54 00 00`. **Freeze ≈ 115–121 frames (~1.9–2.0 s)** — Pac-Man and all
   four ghosts remain visible, fully static (in this state even sprite animation and
   energizer blink stop, since those run inside the state-3 handler).
2. **Maze flash**: 8 alternating states (`#09E8` white via task #01 param 2, `#09FE`
   normal blue via param 0 — the "color maze white" param is documented at the task
   table, `0x23AA`), each holding on `42 00 00`. Phase-locked to the 6-frame clock this
   is **12 frames per color phase** (first phase 7–12), i.e. **4 white flashes,
   white↔blue period 24 frames (~2.5 Hz), ~96 frames total**. The **ghosts are erased**
   at the first flash state (`#267E`); **Pac-Man stays on screen** throughout the
   flashing.
3. `#0A0E`: maze, color RAM, actor RAM and sprites all cleared; hold `43 00 00` =
   **13–18 frames of blank screen**.
4. States `0x20/0x22/0x23` (~3 frames): cut-scene if scheduled, else level counter++,
   pellet map reset, then `jp #0988` → the standard **READY! phase (115–120 frames)**
   of item 4.

**Last dot → control on next level ≈ 345–360 frames ≈ 5.7–5.9 s** (no cut-scene).

Confidence: **ROM-derived** (flash count/period exact; the 7–12-frame first flash
phase is the only jitter).

## 6. Starting lives & extra-life score (DIP defaults)

**Resolved: 3 lives, bonus life at 10,000 — the arcade defaults.**

- S3 MAME `INPUT_PORTS_START( pacman )`: `PORT_DIPNAME( 0x0c, 0x08, DEF_STR( Lives ))`
  with `0x08 = "3"` (options 1/2/3/5, SW:3,4); `PORT_DIPNAME( 0x30, 0x00,
  DEF_STR( Bonus_Life ))` with `0x00 = "10000"` (options 10000/15000/20000/none,
  SW:5,6).
- ROM decode confirmed in S1 at `#26D0` (task #14 reads DSW1 at 0x5080): lives =
  `((dsw>>2)&3)+1`, with 4 promoted to 5 → {1,2,3,5}; bonus table at `0x2728` =
  `10 15 20 FF` (BCD thousands: 10000/15000/20000/none), stored to `4E71`.

Confidence: **ROM-derived + MAME defaults** (MAME's default mask = the factory
switch setting).

## 7. Initial actor facing directions

**Resolved (ROM-derived).** Init block task #04 (`#253D`, param 0 — used for game
start, every life, and every new level via state 9; S1 bytes at `0x25C1–0x2607`):

| actor | direction vector | orientation byte (`4D2C–4D30`) | facing |
|---|---|---|---|
| Blinky | `#0100` → X-change +1 | 2 | **LEFT** |
| Pinky | `#0001` → Y-change +1 | 1 | **DOWN** |
| Inky | `#00FF` → Y-change −1 | 3 | **UP** |
| Clyde | `#00FF` → Y-change −1 | 3 | **UP** |
| Pac-Man | `#0100` | 2 | **LEFT** |

(Orientation codes 0=right, 1=down, 2=left, 3=up — S2 RAM map.) Start pixel positions
in the same block confirm the dossier's geometry: Blinky (X=0x80, Y=0x64) = row 14
straddling cols 13/14; Pinky (0x80, 0x7C) = house center; Inky (0x90, 0x7C) = straddling
cols 11/12; Clyde (0x70, 0x7C) = straddling cols 15/16; Pac-Man (0x80, 0xC4) = row 26
straddling cols 13/14. Every ghost that exits the house (and Blinky re-emerging as
revived eyes) is set to face **LEFT** at the moment it reaches the outside position
(`0x0C74–0x0C8D` etc.).

Confidence: **ROM-derived.**

## 8. Scatter target tiles & eyes house-return target (the one-row question)

**Resolved definitively from ROM constants — the dossier diagram's measurement is
correct; the common secondary values are one row off.**

Raw ROM constants (S1, ghost AI task handlers; `DE = (X, Y)` internal tile pair):

| target | address | instruction | internal (Y, X) |
|---|---|---|---|
| Blinky scatter | `0x274B` | `ld de,#221d` | (0x1D, 0x22) |
| Pinky scatter | `0x2781` | `ld de,#391d` | (0x1D, 0x39) |
| Inky scatter | `0x27BE` | `ld de,#2040` | (0x40, 0x20) |
| Clyde scatter | `0x2806` | `ld de,#3b40` | (0x40, 0x3B) |
| Eyes (all four) | `0x2842/0x286C/0x2896/0x28C0` | `ld de,#2e2c` | (0x2C, 0x2E) |

Coordinate mapping (derived from the ROM's own pixel→tile converter `#2018`:
`tileY = (pxY>>3)+0x20`, `tileX = (pxX>>3)+0x1E`): **row = internalY − 0x1E,
col = 0x3B − internalX** in the dossier's 28×36 grid (col 0 = left, row 0 = top score
row). Anchors verified: house-center tile (0x2F, 0x2E) → (13, 17); above-door tile
(0x2C, 0x2E) → (13, 14); Pac-Man start tile (0x38, 0x2E) → (13, 26). All match the
dossier's measured geometry exactly.

Therefore, in 28×36 grid coordinates (col, row):

| ghost | scatter target | note |
|---|---|---|
| Blinky | **(25, −1)** | one row *above* the visible screen |
| Pinky | **(2, −1)** | one row *above* the visible screen |
| Inky | **(27, 34)** | bottom HUD row |
| Clyde | **(0, 34)** | bottom HUD row |
| Eyes return | **(13, 14)** | tile above the left half of the door |

This **confirms `dossier-mechanics.md` §8.1** (diagram-measured values) and refutes the
widely copied (2,0)/(25,0)/(0,35)/(27,35) variant (e.g. implicit in S6's diagram
reproductions), which is exactly the one-row misreading described there. Behavioral
difference is negligible (targets unreachable) but the ROM values are the above.

Eyes-return detail: the tile target (13,14) is used only for pathfinding (via the
frightened-movement task #0C–#0F, taken because an eaten ghost's *blue flag stays set
while dead* — cleared only for alive ghosts at fright end, `0x139D–0x13C4`, and on house
arrival). Arrival is detected by **pixel equality** with (X=0x80, Y=0x64) (`0x10C6`),
then the ghost descends into the house at 2 px/frame to Y=0x80 (`0x10D2`, `cp #80` at
`0x10EB`), Inky/Clyde shift sideways to X=0x90/0x70, the ghost is revived, and it rises
again at house speed. When no eyes remain in flight the eyes sound stops (`#1101`).

Confidence: **ROM-derived.**

## 9. Fruit display duration

**Resolved: not a PRNG — clock-phase aliasing. 9.0–10.0 s (541–600 frames), uniform in
the phase of the global 1-second clock.** ROM-derived.

`#0EAD` (S1): fruit appears when the dot counter hits 70 (`cp #46`) and again at 170
(`cp #AA`); position written as pixel pair `#8094` → (X=0x80, Y=0x94) = **row 20,
straddling columns 13/14** — same x as Pac-Man's start; this converts the mechanics
doc's §1.3 *inference* into ROM fact. The despawn is timed task `8A 04 00` (`0x0EF8`):
**10 ticks of the 1-second unit**. Because ticks land on global 60-frame boundaries,
expiry occurs 9 full seconds plus the fraction remaining to the next boundary —
"always between nine and ten seconds" (dossier wording, S4) with **no randomness
beyond spawn-frame phase**.

Implementation: either replicate the global 60-frame clock (authentic), or draw
uniform 541–600 frames (equivalent distribution given uniform spawn phase).

The fruit-score sprite after eating a fruit is cleared by its own task (#05) — see
item 11 note.

Confidence: **ROM-derived.**

## 10. Pinky's exit timing at game start; house-bounce speed

**Resolved (ROM-derived).**

- **No scheduled delay for Pinky.** Release checks (`#2069` Pinky, `#208C` Inky,
  `#20AF` Clyde — S1) run every gameplay frame; a ghost is released when its personal
  dot counter ≥ its limit (`cp (hl); ret c`). Level-1 limits (table `0x0843`, selected
  per level): **Pinky 0, Inky 30, Clyde 60**; post-death global-counter limits **7/17/32**
  (`cp #07/#11/#20` inline) — all matching the dossier. With limit 0, Pinky's check
  passes on the **first gameplay frame**; he starts rising on the next house-movement
  frame and never bounces. His climb (Y 0x7C → 0x64 = 24 px at 0.5 px/frame) takes
  **48 frames ≈ 0.8 s** from control-start until he stands on tile (13,14) facing left.
- **House bounce/exit speed: 0.5 px/frame (= 40% of "100%").** All in-house movement,
  door-transit *outward*, and Blinky's post-revival re-exit run through `#0C42` (called
  once per frame), which moves 1 px only on frames where the rotating bit mask `4D94`
  carries; `4D94` is always initialized to **0x55** (`#268B`) = alternate frames.
  Corroboration: S5 models pacing/leaving-home speed with the level-1 tunnel pattern
  `0101…` (same 0.5 px/frame).
- Bounce geometry: ghosts oscillate vertically between pixel Y=0x78 and Y=0x80
  (`0x0CA0/0x0CA5`), i.e. **±4 px around the house-center row (17)**; full cycle
  32 frames. Start Y=0x7C is the exact center of the swing. Exit path: align to
  X=0x80 at 0.5 px/frame, then rise to Y=0x64, then turn LEFT with tile set to (13,14).
  (Eyes *entering* descend at 2 px/frame — item 8.)
- House movement is **paused during the ghost-eaten pause** (`4DA4` gate at `0x0C42`)
  but **continues during the death freeze**.

Confidence: **ROM-derived.**

## 11. Frightened-mode score sprite display time

**Resolved: identical to the ghost-eaten pause — 55–60 frames (≈1 s).** The 200/400/800/
1600 sprite *is* the eaten ghost's sprite slot for exactly the duration of the `4A 03 00`
task (item 1); when the task fires the same slot switches to the eyes sprite. There is no
separate timer. (Sprites 0x28–0x2B, color 0x18, at the eaten ghost's position; Pac-Man
is hidden underneath it for the same interval.)

Related: the score sprite shown after eating a **fruit** is cleared by task #05
(`#100B`) on its own timed task; its scheduling byte sits in the fruit-eat path (S2
shows Ms. Pac's at `0x19C4`; the original's fruit-score display is likewise ~2 s —
timer byte not individually verified in S1). Mark the fruit-score duration
**well-documented secondary / approximation (recommend 2.0 s)**.

Confidence: **ROM-derived** (ghost score); fruit-score sprite duration approximation
as noted.

## 12. Other §8 items resolved en route

- **Eyes vs. red zones (§8.4):** resolved — eaten ghosts keep their blue flag while
  dead, so at decision tiles they take the frightened/dead AI path (`0x1C05`), which
  never performs the red-zone tile-color test; the `cp #1A` color check (`0x1C12`) that
  implements the red zones only runs for non-blue, alive ghosts. **Eyes (and frightened
  ghosts) ignore red zones**, exactly as the mechanics doc guessed. Note the ROM's
  red-zone mechanism: on tiles whose color RAM byte is 0x1A the ghost AI task is simply
  not queued, so the ghost carries on with its previously chosen direction (net effect =
  the dossier's "may not choose to turn up").
- **Frightened-ghost eating collision (§8.4):** the same tile-equality test as death
  (`#171D` compares tile pairs, setting both `4DA4` and `4DA5`; blue flag then converts
  death → eat, `0x176C–0x1788`), **plus** a second, energizer-active-only proximity test
  (`#1789`): unsigned per-axis pixel difference < 4 on both axes also counts as eating a
  blue ghost. Implement tile test for death; tile OR near-proximity for eating.
- **Exact fruit tile (§8.4):** ROM pixel (0x80, 0x94) → row 20, straddling cols 13/14
  (item 9). Mechanics-doc §1.3 inference confirmed.
- **Ghost-eaten score ladder reset:** `4DD0` (count of ghosts eaten this energizer) is
  cleared both when fright ends by timer and when all four are eaten (`0x13D2`).
- **Force-release ("inactivity") timer:** counter compared to `4D95` limits from table
  `0x0873` = 240/240/180 frames — at one increment per frame (`#13DD`, called once per
  frame) that is **4 s (early levels) / 3 s**, confirming the dossier (and correcting
  S2's own "2 seconds" comment, which assumed the wrong tick rate).
- **First-start actor visibility:** actors appear only in phase B of the first READY!
  (item 4); the "PLAYER ONE" phase has no sprites on screen.
- **Ghost animation during pauses:** leg/eye sprite animation (8-frame cycle, `4DC0`)
  keeps running during the ghost-eaten pause and the 60-frame death freeze; it stops
  during the level-complete freeze.
- **Start jingle exact audio length:** not extracted (sound ROM tables out of scope) —
  **documented approximation**: it fills the ~4.1–4.3 s two-phase hold of item 4.

## 13. Deviation ledger for the implementation

Label as *exact ROM behavior*: items 1, 2, 3, 5 (counts/periods), 6, 7, 8, 9
(mechanism), 10, 11 (ghost score), 12.

Label as *documented approximation* (ROM value is a phase window; pick the
phase-locked/upper value unless emulating the global clock):

| timer | window | recommended fixed value |
|---|---|---|
| ghost-eaten pause | 55–60 f | 60 f |
| READY! (all) | 115–120 f | 120 f |
| first-start "PLAYER ONE" phase | 133–138 f | 138 f |
| level-complete freeze | 115–121 f | 120 f |
| first maze-flash phase | 7–12 f | 12 f (rest are 12) |
| post-flash blank | 13–18 f | 18 f |
| fruit duration | 541–600 f | uniform random 541–600 f |
| fruit-score sprite | unverified | 120 f (≈2 s) |
| start jingle audio | unverified | fit inside 4.2 s hold |
