# Validation 03 — live gameplay acceptance run

Date: 2026-09-29 · Environment: Ghostty (TERM_PROGRAM=ghostty) inside cmux,
macOS Darwin 27.0.0. Release build `cargo build --release --locked` at commit
fa29a31; 222 tests green, CI green.

## Method

- All keystrokes were delivered to the real app in the terminal via
  `cmux send` / `cmux send-key` (real TTY input). Screenshots via
  `docs/validation/snap_cmux.sh` (activates the window briefly, since
  occluded Ghostty panes freeze their pixels).
- Timing: wall seconds = tick / 60.606061. The first READY! lasts 258 ticks
  (~4.26 s). Launch-to-tick-0 overhead measured between ~0.2 s and ~1.1 s
  across runs — it affects capture timing only, never sim behavior; replays
  are tick-deterministic.
- Replay repro: `./target/release/pacmantui --replay
  docs/validation/replays/<file>`, then capture at tick/60.606 s (plus launch
  overhead) after tick 0. The attended run (figures 20–23) is interactive and
  not tick-reproducible.
- Event ticks in the three replays: classic-level1-clear — fruits t=1127 and
  t=2411, LevelCleared t=4557. vertigo-level1-clear — fruits t=1351 and
  t=2950, LevelCleared t=5742. classic-fright-demo — FrightenedStarted t=416,
  FrightenedEnded t=896 (360 fright ticks plus two 60-tick ghost-eaten
  freezes), PacDying t=1009.

## Honesty notes

- The two level-clear replays were AUTHORED by a bot
  (`examples/bot_clear.rs`) issuing legal per-tick inputs only — no state
  cheats. Both were re-verified headlessly at this commit: classic
  cleared=true score=4600 lives=3 ticks=4558; vertigo cleared=true score=6360
  lives=3 ticks=5743 — byte-identical to the committed replay files.
- The attended run (figures 20–23) is real keyboard input steered live.
- For figure 24 the game was PAUSED with `p` inside the frightened-flash
  window to freeze the 14-tick white phase for capture (brief resumes step
  the phase). The flash itself runs unpaused in normal play.

## Figures

Evidence files: `evidence/10-*.png` … `evidence/24-*.png`.

- **10 — classic first fruit** (`10-classic-fruit1.png`): classic replay at
  ~t≈1200 (fruit spawned t=1127). First cherry on the corridor below the
  house, score 1340, HUD shows lives ×2 plus the cherry in the fruit strip.
  Blinky/Pinky/Inky are out; Clyde is still housed (pre-90-pellets release
  counter). Verifies fruit spawn placement, HUD, and the house dot-counter
  live.
- **11 — classic fright chain + second fruit** (`11-classic-fright-chain-fruit2.png`):
  classic replay at ~t≈2470 (fruit spawned t=2411), during an active fright:
  two blue ghosts, an eyes sprite returning to the house, cyan "400" chain
  popup, score 3220, second cherry present. One frame verifies the second
  fruit spawn, frightened rendering, the 200-400-… eat chain, and eyes
  return.
- **12 — classic last dot** (`12-classic-last-dot.png`): the tick before the
  final dot — score 4590, exactly ONE dot left with Pac-Man mouth-open at it,
  the ghosts at the positions the clear freeze will hold. Verifies dot
  accounting down to the 244th pellet.
- **13 — classic clear sequence** (`13-classic-clear-flash.png`): the
  level-clear sequence just after the final dot (LevelCleared t=4557) — maze
  empty, score 4600 (equal to the headless bot result), Pac-Man visible. The
  four ghosts are still shown at their freeze positions: this frame lands
  inside the initial 120-tick freeze of the 234-tick sequence, before ghosts
  are hidden and the maze flashes (compare figure 17, captured in the later
  phase).
- **14 — classic level-2 READY!** (`14-classic-level2-ready.png`): fresh full
  pellet board, score carried at 4600, HUD fruit strip now cherry+strawberry
  (Table A.1 symbol progression). Verifies level progression and score carry.
- **15 — pause overlay** (`15-pause-overlay.png`): PAUSED overlay mid-level-2
  (score 4670), reached via Esc — Esc pauses, a second Esc goes to the menu,
  per the app's documented key map; `q` from pause quits cleanly to the
  prompt. Verifies the pause overlay over live level-2 state.
- **16 — Vertigo first fruit** (`16-vertigo-fruit1.png`): re-captured on the
  fixed build (see "Post-release fix" below) — vertigo replay shortly after
  the first fruit (t=1351), score 1300, cherry on the corridor below the
  low-center house; Inky/Pinky/Blinky are out, Clyde still housed. The HUD
  now sits in its own padded bands, clear of the maze. HIGH SCORE reads 2020,
  persisted from the user's own play session — incidentally live evidence of
  high-score persistence.
- **17 — Vertigo clear flash** (`17-vertigo-clear-flash.png`): re-captured on
  the fixed build — the vertigo clear sequence past tick 120: empty board,
  final score 6360 (equal to the headless result), ghosts hidden, Pac-Man
  visible. The bottom corridor, previously overprinted by the lives/fruit
  strip, is now fully visible.
- **18 — Vertigo level 2** (`18-vertigo-level2-ready.png`): re-captured on
  the fixed build — fresh vertigo board immediately after the level-2 READY!
  banner cleared (capture timing kept missing the ~2-second window), Pac-Man
  at his row-7 spawn with the start-configuration ghosts, cherry+strawberry
  HUD strip, score 6360 carried. The banner's placement on padded maps is
  asserted by the headless test `custom_map_banners_shift_into_maze_band`
  (tests/render_hud_padding.rs) instead of a screenshot.
- **19 — menu after Vertigo replay** (`19-menu-after-vertigo-replay.png`):
  main menu reached by Esc (pause) → Esc (menu) from the vertigo replay. The
  menu shows MAP CLASSIC because `--replay` sessions deliberately never write
  the persisted settings — `settings.map` is written only on menu selections
  (src/app/mod.rs) — so the menu boots with the last MENU-selected map.
  Verifies the return-to-menu path and replay isolation from persistence.
- **20 — attended respawn** (`20-realkeys-respawn-ready.png`): attended
  real-keys run — death, then respawn READY! at score 390 with one life
  consumed and the eaten corridors visibly empty. Verifies pellet
  preservation across death, live.
- **21 — resized small, paused** (`21-resize-small-paused.png`): mid-pause,
  the OS window resized 1727x998 → 1200x700 pt — the game re-letterboxed at a
  smaller integer scale, PAUSED state intact (score 520), pure black
  letterbox. Deliberately a full-window capture, so the neighboring terminal
  pane is visible.
- **22 — resize restored, paused** (`22-resize-restored-paused.png`): window
  restored to 1727x998 — identical game state at full scale. State survives
  both resizes.
- **23 — restart** (`23-restart-ready.png`): `r` while paused — score 00,
  HIGH SCORE 520 retained, fresh board, READY!.
- **24 — frightened white flash, paused** (`24-fright-white-flash-paused.png`):
  classic-fright-demo replay, frozen via pause inside the flash window: a
  white-bodied, red-faced flash sprite by the house door, lingering cyan
  "400" popup, score 1010. Cropped from a full-window capture with `sips`
  (the game pane had moved after a pane recreation).

## Post-release fix: custom-map HUD overlap

- After this acceptance run shipped, the user reported in live play that on
  Vertigo the HUD overprinted the maze — score text on the top
  border/corridor, the lives+fruit strip on the bottom corridor. A larger
  window made it obvious.
- Root cause: the gameplay frame was sized directly from the map grid
  (src/render/mod.rs took `map.width() x map.height()`), while `draw_hud`
  paints fixed frame rows — the top two and the bottom two. Classic masked
  the coupling by embedding 3 dead rows above and 2 below the maze in its
  own 28x36 grid; Vertigo's all-maze 32x26 grid embeds none, so the HUD
  landed on maze tiles. It slipped through the original acceptance because
  at the smaller review scale the overlap read as adjacency.
- Fix (render layer only): HUD padding is derived from the map's open-tile
  bounding box — pad_top/pad_bottom = 3/2 minus whatever dead rows the grid
  already embeds — the maze layer and everything maze-anchored shift down by
  pad_top tile rows, and the HUD stays anchored to the frame. Classic
  derives (0,0) padding and was proven byte-identical across the change via
  six FNV-1a hashes (blue and white-flash maze layers plus four composed
  frame variants), captured before and after. Vertigo's frame is now 32x31
  tiles (256x248 px) with the maze band at tile rows 3..28.
- Regression tests: tests/render_hud_padding.rs
  (`custom_map_hud_gets_padded_rows`, `custom_map_banners_shift_into_maze_band`,
  `classic_map_needs_no_padding`, `hud_pad_rows_from_open_bounding_box`).
  Figures 16–18 above were re-captured on the fixed build and lead-verified
  live (HUD in its own bands, maze/actors/door correctly shifted, letterbox
  recentered).

## Checks without figures

- Three separate Ctrl-C exits (a stale replay instance from a previous
  session, the attended run mid-game, and a replay run) each restored a clean
  shell prompt with no residual process (`pgrep` empty) and no terminal
  corruption.
- `p` toggles pause/resume: figures 15/21/22 show pause; resumed gameplay was
  verified between them.
- Quit paths Esc → menu → `q` and pause → `q` both restore the prompt.
- Mid-session the cmux playtest pane was closed externally; a replacement
  pane was created (`cmux new-pane`) and all subsequent runs used it. One
  capture batch (w1–w4) was invalidated by the switch and excluded.
- After this document was written, the full menu tree was exercised live
  end-to-end with real keys: MAP cycled CLASSIC→VERTIGO→CLASSIC and
  DIFFICULTY NORMAL→HARD→NORMAL (right-arrow cycling, selections restored),
  the HIGH SCORES screen and the CONTROLS screen were each opened and
  exited, and the menu QUIT path exited to a clean prompt (it persists
  settings on quit — src/app/mod.rs `MenuAction::Quit`). Screen captures of
  the sub-screens were not possible at that point because the cmux window's
  visible tab was occupied by the user's own session (input reaches
  background surfaces; pixels of an invisible tab cannot be captured). The
  menu itself is evidenced by figure 19 and by
  evidence/03-menu-classic-normal.png.
