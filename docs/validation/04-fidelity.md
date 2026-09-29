# Validation 04 — arcade-fidelity comparison

Date: 2026-09-29 · Scope: visual fidelity of the classic-map render vs arcade
reference imagery, plus Vertigo vs its own spec. Environment-neutral: all
comparisons were made in the game's 224x288 logical pixel space (the renderer
letterboxes that screen at an integer scale over black, so frames were
registered and downsampled before comparing; absolute frame sizes are
irrelevant).

## Reference material (third party — NOT redistributed in-repo)

Fetched at validation time from the Pac-Man Dossier companion site and kept
only in the session scratchpad:

- `pacman.holenet.info/lvl1.png` — full level-1 arcade screen (336x432, a
  1.5x reproduction with antialiased ramps; exact shades compared with that
  in mind).
- `pacman.holenet.info/StartPositions.png` — actor start arrangement.
- `pacman.holenet.info/Tiles.png` — tile grid / walkable-tile overlay, used
  to cross-check pellet placement and the dot-free zones.

Our side: committed evidence `evidence/03-menu…07-death-animation.png` plus
fresh full-palette frames from this session's live runs (`k7` fresh classic
level-1 with READY!, `c1` classic mid-game with fruit, `c5` classic level-2
READY!, `v1`/`v5` Vertigo; session scratchpad, not committed).

## Method

Both level-1 start screens (lvl1.png and k7) were registered onto the 28x31
board tile grid by least-squares fit of the pellet lattice (phase fit +
top-left-dot anchor; recovered our scale/origin exactly), then:

- pellet blobs classified into dots/energizers and mapped to tiles;
- wall masks resampled to logical 224x288 and XOR-compared with a ±2 px
  tolerance, sprite-occluded areas masked (occlusion is symmetric — both
  frames show the identical start arrangement);
- sprite centroids measured in logical px for start positions;
- colors read as exact RGB off our frames, dominant-color histograms off the
  reference.

Vertigo (`v5`, a fresh board) was checked the same way against the map's own
declared design: the grid, spawn, house, and fruit spec in
`maps/custom.pmtoml` (header comments + tables).

## 1. Maze silhouette — agreement (exact), one wall-art deviation

- Wall layout and corridor topology match tile-for-tile. After registration,
  the ±2 px structural diff over the whole board is empty except for the
  four tunnel side pockets (below).
- Ghost house: position, size, and the 2-tile door gap at tiles (13,14) of
  row 15 match; the door is a pink horizontal bar spanning the gap in both.
- Tunnel row 17 is open through both screen edges in both.
- Deviation (wall art only, topology unaffected) — **tunnel side pockets**
  (cols 0–5 / 22–27, rows 12–22): the arcade draws these out-of-bounds
  blocks with double outlines whose lines run open off the screen edge;
  ours draws single-outline closed shapes and a boundary line that closes
  the pocket mouths ~2 px inside the screen edge. Cause: the generic
  contour-tracing wall renderer (`src/render/scenes.rs`), which must handle
  arbitrary custom maps. Not documented anywhere as a deliberate
  arcade-look deviation — flagged as a finding below.
- Related: our outermost border line sits ~2 px inside the screen edge on
  all sides (arcade: flush at x=0/x=223). Same renderer cause; cosmetic.

## 2. Pellets and energizers — agreement (exact)

- 240 dots detected in both frames, at exactly the same 240 tiles (empty
  set difference both directions).
- 4 energizers in both, at tiles (1,6), (26,6), (1,26), (26,26); circle
  diameter ~7–8 px in both (≤1 px apart, within measurement noise of the
  smoothed reference).
- Dots are 2x2 px at tile centers in both. The dot-free zones (house
  perimeter corridors, tunnel band, READY!/fruit row 20) follow from the
  exact dot-set match and agree with Tiles.png.

## 3. Colors — agreement (ours matches the canonical palette exactly)

Measured off our full-palette frames vs the reference reproduction:

| element | ours (exact) | lvl1.png (dominant) | note |
|---|---|---|---|
| maze wall | #2121DE | ~#1F1FF2 core + AA ramp | ours is the canonical arcade value; ref is smoothed/brightened |
| background | #000000 | #000000 | |
| dots | #FFB8AE | #FFDED2 (cream) | ours = documented arcade pellet color and = the ref's own energizer color; the ref reproduction draws dots lighter |
| energizers | #FFB8AE | #FFB8AE | exact match |
| Pac | #FFFF00 | #FFFF00 | |
| Blinky | #FF0000 | #FF0000 | |
| Pinky | #FFB8FF | #FFB8FF (+AA) | |
| Inky | #00FFFF | #00FFFF | |
| Clyde | #FFB852 | #FFB851 | 1-step AA difference |
| door | #FFB8DE | ~#FFCAFF pale pink | same position/size; slightly pinker in ours |
| frightened body | #2121FF | — (no fright frame in refs) | matches the documented arcade fright blue |
| frightened face | #FFFFFF | — | white features (evidence 05) |
| ghost-score popup | #00FFFF cyan "200" | — | arcade-style cyan score (evidence 05) |

Committed evidence 05/06 frames are uniformly dimmed by a ~0.70 capture
factor; their colors decode exactly to the palette values above (e.g.
(23,23,179) → #2121FF). The fresh session frames are full-palette.

## 4. Sprites — deliberate original art, in-style (documented)

Per `src/render/sprites.rs` module doc and TRACEABILITY.md D10, all sprite
art is original, "in the arcade style, inspired not pixel-identical". How it
actually differs:

- Ghosts: same silhouette class (domed body, scalloped skirt, two large
  white eyes with blue pupils, ~14 px) but the skirt scallop pattern and
  eye/pupil proportions are our own; simplified pixel detail vs ROM art.
- Pac: ~13 px yellow disc with wedge mouth in both; start pose is the full
  closed disc in both; ours has slightly blockier mouth edges. Death
  animation (evidence 07) is the arcade-style collapsing arc, original
  pixel pattern.
- Frightened: blue body with white eyes + wavy mouth (evidence 05), same
  reading as the arcade sprite, simplified features. The white-flash phase
  is captured in `evidence/24-fright-white-flash-paused.png` (03-live-
  gameplay.md figure 24, frozen via pause inside the flash window):
  white-bodied, red-faced flash sprite, the arcade's flash color scheme.
  The fetched references contain no fright/flash frame, so no side-by-side
  was made.
- Fruit: cherry = two flat red circles + highlight, tan stem, green leaf;
  strawberry = red with white seeds and green crown (c5). Both instantly
  read as their arcade counterparts; the arcade sprites are more heavily
  shaded. Positions are exact: classic in-maze fruit sprite center measured
  at (112,164) — the arcade fruit point (fruit-body centroid offset was
  bit-identical, +3.9/+2.8 px, between classic and the spec-verified
  Vertigo fruit).
- READY! is yellow, centered on row 20, in both. HUD/READY glyphs use an
  original blocky arcade-style font; shapes are near-identical to the
  reference at a glance, metrics (8 px cell) match.

## 5. HUD — agreement, single-player scope deviation

- Top: `1UP` with score right-aligned below it, `HIGH SCORE` centered with
  value below — same rows (0–1), same columns as the arcade layout. (The
  reference frame happens to show neither `1UP` — caught mid-blink — nor
  `2UP`.)
- Deviation (scope, deliberate): the arcade reserves a `2UP` area; pacmantui
  is single-player and renders none. Documented here.
- Lives strip: bottom-left, row 34, left-facing Pac icons, two shown for the
  3-life start (in-play life not shown) — matches the reference exactly.
- Fruit strip: bottom-right, row 34, newest fruit in the rightmost slot:
  level 1 shows cherry (k7 = ref), level 2 shows cherry then strawberry with
  strawberry rightmost (c5) — arcade ordering.
- Post-release: a HUD/maze overprint defect on all-maze custom grids
  (Vertigo) was found in post-release live play and fixed by
  bounding-box-derived frame padding, with classic byte-identity preserved
  (hash-verified); see 03-live-gameplay.md "Post-release fix".

## 6. Start positions — agreement (sub-pixel)

Sprite centroids in logical px, ref vs ours (Pinky's ref value is polluted
by the door's pink pixels; her clean position is the house center):

| actor | ref | ours |
|---|---|---|
| Blinky | (112.3, 116.8) | (112.1, 117.0) |
| Inky | (96.0, 141.3) | (96.1, 141.0) |
| Pinky | (111.6, 137.5)* | (112.1, 141.0) |
| Clyde | (127.7, 140.9) | (128.1, 141.0) |
| Pac | (112.5, 211.5) | (112.0, 211.5) |

Arrangement matches StartPositions.png: Blinky outside, centered above the
door; Inky/Pinky/Clyde left/center/right in the house; Pac centered on the
energizer row (26), full-disc pose, READY! below the house.

## 7. Vertigo vs its own spec (maps/custom.pmtoml) — agreement (exact)

Checked `v5` (fresh board) against the declared grid with the same pipeline:

- 32x26 landscape grid confirmed; board fills the logical screen, so the
  HUD text overlays the top corridor rows (classic has dedicated HUD rows;
  map-format consequence, noted).
- Pellets: grid declares 336 dots + 4 energizers; 335 dots + 4 energizers
  detected, and the single missing dot is under a sprite/HUD-text occlusion
  — zero unexplained differences in either direction.
- Energizers at the declared corner tiles (1,2), (30,2), (1,23), (30,23).
- Vertical tunnels: outer-wall gaps at columns 3 and 28 on both the top and
  bottom edges, as declared.
- Walls: no blue in any declared open tile; every declared wall tile carries
  its outline except interiors of multi-tile slabs (outline rendering —
  expected). Ladder motif and the full-width row-4 crossing visible.
- House low-center with pink door at its top; spawns measured at
  Pac (128.0,59.5) [spec (128,60)], Blinky (128.1,108.9) [(128,108)],
  Pinky (128.1,132.9) [(128,132)], Inky (112.1,132.9) [(112,132)],
  Clyde (144.1,132.9) [(144,132)] — all within 1 px (skirt-frame asymmetry).
- Fruit (v1): cherry below the house at spec center (128,156) (red-body
  centroid (131.9,158.8), identical offset to the classic cherry).

## Findings

1. **F-VIS-1 (for the lead — undocumented cosmetic deviation):** tunnel
   side-pocket wall art differs from the arcade (single-outline closed
   shapes + pocket mouths closed at the screen edge, vs double-outline open
   to the edge), and the outer border sits ~2 px inside the screen edge.
   Both stem from the generic contour wall renderer in
   `src/render/scenes.rs`. Layout, topology, and pellets are unaffected.
   Nothing in TRACEABILITY.md or the render docs records this as
   deliberate; recommend either documenting it alongside D10 or special-
   casing the classic map's tunnel band.
   **Resolved — documented:** now recorded as deviation D15 in
   docs/plan/TRACEABILITY.md (accepted as part of the original-art scope
   alongside D10; no renderer change).
2. ~~Evidence gap: no captured frame shows the frightened white-flash
   phase.~~ **Closed:** `evidence/24-fright-white-flash-paused.png` was
   captured after this analysis (game paused inside the flash window;
   white-bodied, red-faced flash sprite — see
   docs/validation/03-live-gameplay.md figure 24).
3. Reference caveat: lvl1.png is a smoothed 1.5x reproduction whose dots are
   drawn cream (#FFDED2) while its own energizers are #FFB8AE; our render
   uses #FFB8AE for both, the documented arcade pellet color. Not counted
   as a deviation on our side.

No unintended layout, pellet, position, or palette mismatches were found.

## Verdict

The classic-map render is tile-exact against the arcade reference in maze
layout, pellet/energizer placement, dot-free zones, start positions, HUD
layout, and fruit/lives strips, and palette-exact against the canonical
arcade colors. Deviations are: the documented original sprite art (D10), the
single-player HUD (no 2UP), and one cosmetic wall-art
difference in the tunnel pockets (F-VIS-1, since documented as D15). Vertigo matches its declared
design exactly. Reference images were used for comparison only and are not
redistributed in this repository.
