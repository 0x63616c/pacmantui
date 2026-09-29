# Independent Review: `sim` module vs reference documentation

- **Reviewer**: independent (did not author any of this code)
- **Date**: 2026-09-29
- **Commit under review**: `61155b06ecde5789f6f72e5c6e6647bdd1e2aa65`
- **Scope**: `src/sim/{mod.rs,actors.rs,timings.rs}` and `tests/sim_*.rs` (incl.
  `sim_helpers.rs`) vs `docs/research/dossier-mechanics.md` ("dossier §N"),
  `docs/research/arcade-supplements.md` ("supp §N"), `docs/research/tables.md`/
  `tables.json`, `docs/map-format.md`, `docs/plan/IMPLEMENTATION.md`, and
  `docs/plan/review-rules.md` finding F2. Rules-table data itself was already
  verified by review-rules.md and is treated as verified here.

## Verdict: APPROVED-WITH-NOTES

The engine's mechanics match the cited documentation everywhere I checked, all
seven interpretation decisions are the best-supported readings of the sources,
and `cargo test --all-targets --all-features` is green (52 sim tests across 8
files; full suite exit 0). I re-derived every numeric expectation in the seven
in-scope test files from the docs alone (~145 individual values; inventory at
the end) and found the implementation correct in every case — but one test's
central expectation (F1) is mis-derived from the dossier and passes only by
coincidence, and one freeze behavior (F2) deviates from a cited ROM fact.
Nothing blocks: the sim itself is faithful.

## Method

- Read every line of the three sim sources, `types.rs`, the `rules` and `map`
  consumer APIs (`scale_dot_threshold`, `warp`, `is_tunnel`, spawn/target
  getters), both shipped `.pmtoml` maps, and all four test mini-maps.
- Re-derived each test expectation by hand from the docs: Fix8 accumulator
  stepping (`px(t) = floor(speed·t/256)`, speed = `round(320·pct/100)` — exact
  for every table percent), grid geometry from the map sources, timer values
  from Table A.1 (×60 ticks/s, per review-rules F1's sanctioned reading) and
  from supp §13's phase-locked column. Tick-by-tick traces were checked against
  the sim's documented per-tick phase order (mod.rs module docs), not against
  test comments.
- Verified the RNG-dependent claim in `fruit_expires_within_the_rom_window`
  (seed 19 ⇒ first xorshift draw ⇒ 544 ticks) by reimplementing the seed fold +
  xorshift32 in Python: confirmed 544, expiry window 541–600 = supp §13.
- Ran the full suite plus each `sim_*` test binary individually
  (10+12+5+7+7+5+3+3 = 52 passed, 0 failed).

## Findings

| # | Severity | Location | Finding |
|---|----------|----------|---------|
| F1 | **MAJOR** | `tests/sim_house.rs:54-110` (`classic_level1_inky_at_30_dots_clyde_at_60`) | **Clyde's expectation is mis-derived; the assertion passes for the wrong reason.** Dossier §3.10: only the most-preferred housed ghost's counter increments, and a released counter "deactivates (not reset)" — so on level 1 Clyde's personal counter starts counting only after Inky exits at 30, and his limit of 60 is reached at **90 total pellets**, not 60. The implementation is correct (`on_pellet_counters` increments only `first_housed()`; at pellet 60 Clyde's counter reads 30 < 60). The test's waypoint route eats exactly 60 pellets and then loops a fully-eaten circuit forever, so the `GhostReleased{Clyde}` it observes at `pellets == 60` is actually the **240-tick no-dot force-release** (dossier §3.10, third mechanism) firing after eating stops — the pellet count merely still reads 60. The comment "the release event lands on the very tick of the 30th/60th pellet" is false for Clyde, and the test cannot distinguish the correct implementation from a wrong one that counts every dot into every housed counter (which would also release Clyde at pellet 60, via the personal path). **Action**: rework the scenario — park Pac-Man dot-free after pellet 60 for < 240 ticks and assert Clyde still `InHouse`, then eat on to pellet 90 and assert the release lands there (or assert the release tick is ≥ 240 ticks after the 60th pellet's tick, proving it was the timer). |
| F2 | MINOR | `src/sim/mod.rs:613-631` (`DeathFreeze` arm) vs supp §10 | During the 60-tick death freeze the sim moves only `InHouse` ghosts (bounce); a `Leaving` ghost is frozen mid-exit. Supp §10 is explicit that house movement — including "door-transit *outward*" — "is paused during the ghost-eaten pause but **continues during the death freeze**" (`#0C42` is not gated on `4DA5`). Effect is purely cosmetic: `reset_actors` follows the death animation, so no gameplay state depends on it, and the sim gets the ghost-eaten-pause side (paused) right. **Action**: run `house_leave` too for `Leaving` ghosts in the `DeathFreeze` arm, or note the deviation in the module's deviation list. |
| F3 | NOTE | `tests/sim_helpers.rs:76-78` (RING header comment) | The comment says "21 dots + 2 energizers = 23"; the grid actually holds **23 dots + 2 energizers = 25 pellets** (row 1: 8, rows 2–4: 2 each, row 5: 9). No assertion uses the wrong 23 — `sim_elroy.rs` correctly derives from 25 ("With 25 pellets… Elroy 2 when the 5th pellet drops the remainder to 20": 25−5=20 ✓), and the house/scoring tests count eaten pellets independently. Fix the comment. |
| F4 | NOTE | `src/sim/timings.rs:19` (`EXTRA_LIFE_SCORE`) | Dead duplication: the sim awards via `rules.extra_life_score()` (mod.rs:989); the timings constant is referenced by nothing. The comment discloses the duplication, but the two values could silently drift. Remove it or add a static assert. |
| F5 | NOTE | `src/sim/mod.rs` phase-8 docs / `check_collisions` fruit-eat + `update_fruit` | Two small collision conventions are invented without a doc citation (none exists to cite): (a) fruit is eaten by tile equality with `fruit_pos().tile()` — the boundary-straddling classic fruit (112,164) resolves to tile (14,20) only, so the eat point is asymmetric by up to 8 px by approach side; (b) when a same-tile frightened ghost and a non-frightened ghost coincide in one tick, fixed index order decides. Both are benign and consistent with the tile-collision conventions of dossier §4.1; flagged only for the record. |
| F6 | NOTE | Test coverage gaps (no wrong behavior found) | Untested documented behaviors: the §3.9 **exit-right** rule (mode change while housed ⇒ ghost leaves facing Right) — `exit_right` is implemented but no test drives it; the §3.10 global-counter **stays-active-forever** branch (Clyde outside at 32 ⇒ later-eaten ghosts only released by the timer) — only the deactivation branch is tested; the flash-**shrink** path of `fright_is_white` (1 s/3-flash levels, half < 14) — flash counts are tested only at L1's 14-tick half; the Hard-difficulty schedule/limits in sim context (Hard is exercised only via the F2 fruit test). None blocks; all are guarded at the `rules` level or by code reading. |

## Resolutions

| # | Resolution |
|---|------------|
| F1 | **Fixed** — test rewritten as `classic_level1_inky_at_30_dots_clyde_at_90`: a continuous 90-pellet waypoint route keeps every inter-dot gap far below the 240-tick no-dot timer, asserts Clyde still `InHouse` on the 60th pellet, and the same-tick `ate`-flag correlation on each `GhostReleased` proves the counter path (not the timer). |
| F2 | **Fixed** in `src/sim/mod.rs` — the `DeathFreeze` arm now runs `house_leave` for `Leaving` ghosts too, so outward door transit continues during the death freeze (supp §10). Regression test: `leaving_ghost_finishes_door_transit_during_death_freeze`. |
| F3 | **Fixed** — RING header comment corrected to 23 dots + 2 energizers = 25 pellets. |
| F4 | **Fixed** — dead `EXTRA_LIFE_SCORE` const removed from `src/sim/timings.rs`; its ROM provenance note moved to the `rules::Rules::extra_life_score()` doc. |
| F5 | **No action**, as the review concluded — benign conventions, flagged for the record. |
| F6 | The §3.9 exit-right rule is now covered by `mode_flip_while_housed_makes_ghost_exit_right` (tests/sim_house.rs); the other listed gaps remain guarded at the `rules` level or by code reading, per the review. |

## The seven interpretation decisions (all judged sound)

1. **GhostScoreFreeze semantics** — matches supp §1 in every particular:
   fright + scatter/chase timers frozen, Pac-Man hidden and frozen, live
   ghosts frozen, in-house bounce paused (supp §10), prior victims'
   eyes/entering keep moving at 2 px, fruit/popup/anim/blink timers run,
   score-sprite→eyes at tick 60. Verified against the test and the ROM notes.
2. **Fruit duration via sim RNG** — supp §9 explicitly sanctions a uniform
   541–600 draw as distribution-equivalent to the clock-phase aliasing;
   documented as RNG consumption point 2. (Side-effect, disclosed in
   timings.rs: durations repeat per level since the RNG reseeds — acceptable.)
3. **Cornering window 4/3** — engine tile centers sit at pixel offset 4 vs the
   dossier's offset 3, mirroring the 3-pre/4-post split per entry side while
   preserving the window (the whole turn tile) and the 45° double-speed cut.
   A real 1-px deviation, correctly self-documented; the alternative (center
   at offset 3) would ripple through every module. Sound.
4. **Global-counter equality-on-increment + scaled global limits** — the
   equality check at exactly 7/17/32 is what dossier §3.10's
   keep-them-housed trick requires and the impl reproduces it (re-housed
   Pinky/Inky are only ever freed by the timer). Scaling 7/17/32 via
   `scale_dot_threshold` is an extrapolation, but IMPLEMENTATION.md's binding
   "counters" clause covers it and the Vertigo test (round(7·340/244)=10)
   locks it. Sound.
5. **Staggered releases / dead-end reversal / flags discarded on eyes** —
   one-release-per-tick is a disclosed deviation (ROM checks all three each
   frame), gameplay-negligible; the dead-end fallback (`travel.opposite()`)
   is unreachable on the classic maze and the only sane choice on custom
   maps; discarding `reverse_pending` at eyes conversion follows supp §12
   (dead ghosts never run the alive-ghost AI). Sound.
6. **Flash half-period shrink** — Table A.1's flash *counts* are hard data;
   the 14-tick half is an admitted approximation; shrinking the half so
   `flashes·2·half ≤ fright_ticks` preserves the documented counts exactly on
   1-second levels. Best-supported reading. Sound.
7. **Elroy suspension lifts on Clyde→Leaving** — dossier §3.11 "until
   [Clyde] stops bouncing … and moves toward the door" is precisely the
   InHouse→Leaving transition (`release_ghost(3)`); the immediate lift when a
   map spawns Clyde outside is documented and correct (nothing to wait for).
   Sound.

## Mechanics audit summary

| Mechanic | Doc | Impl | Test(s) | Verdict |
|---|---|---|---|---|
| Speeds (MOVEMENT cols, Fix8, ~cols unused) | §2.1, §8.2, A.1 | mod.rs pac_speed/ghost_speed | movement 1, elroy 1 | ✓ (no `pac_dots_speed` use anywhere in sim) |
| Eating pauses 1/3 ticks | §2.4 | timings + move_pac | movement 2–3 | ✓ |
| Buffered turns, cornering, reversal, wall stop | §2.2–§2.3 | actors pac_step/try_turn | movement 4–6, 9 | ✓ (decision 3) |
| Warp (both axes, modulo) | map-format | Map::warp per px step | movement 7–8 | ✓ |
| Ghost decision, look-ahead, tie-break, dist² | §3.4 | choose_dir | targeting 6–8 | ✓ |
| Red zones + fright/eyes exemption | §3.7, supp §12 | choose_dir/choose_frightened | targeting 8–10 | ✓ |
| Per-ghost targets incl. up-overflow, Clyde 8 | §3.6 | chase_target | targeting 1–5, 11 | ✓ |
| Scatter corners / eyes tile | §3.5, supp §8 | map data + target_tile | targeting 5, 12 | ✓ |
| Schedule, flip reversals, fright-frozen timer | §3.2–§3.3 | advance_mode_timers | modes 1–4 | ✓ (420/421 boundary exact) |
| Frightened: duration, flashes, clockwise RNG turn | §3.8, A.1 | energize/choose_frightened | modes 3–5 | ✓ |
| House personal counters / preference order | §3.10 | on_pellet_counters/evaluate_releases | house 1–2 | ✓ impl; **F1 test** |
| Global counter + Clyde quirk | §3.10 | on_pellet_counters | house 5, 7 | ✓ |
| No-dot force release 240/180 | §3.10 | tick_no_dot_timer | house 3 | ✓ |
| Bounce/leave/enter speeds, exit point | supp §8/§10 | house_* fns | house 1, 4, 6 | ✓ (F2: DeathFreeze nuance) |
| Eyes 2 px, revival, Blinky re-exit | supp §2/§8 | move_eyes/house_enter | house 6 | ✓ |
| Elroy stages, scatter override, suspension | §3.11 | elroy_stage/target_tile | elroy 1–3, sequences 2 | ✓ |
| Collision: tile death, pass-through, <4 px eat | §4.1, supp §12 | check_collisions | scoring 1–3 | ✓ (boundary dx=4 exact) |
| Ghost chain 200–1600, resets | §5.1, supp §12 | eat_ghost/energize | scoring 4 | ✓ |
| Fruit spawn/eat/expiry/popup, F2 hard rule | §5.2, supp §9/§11, A.2 prose | spawn_fruit/update_fruit/bonus_spec | scoring 5–7 | ✓ |
| Sequences: READY 258/120, death 60+160, clear 234, freeze 60 | supp §1/§3/§4/§5/§13 | sequence_tick + timings | sequences 1–4 | ✓ |
| Progression, history 7, extra life, L17, 21+ | §5.2–§5.4, A.1 | start_level etc. | sequences 3, 5 | ✓ |
| Determinism: no maps, int math, 2 RNG points, reseed | §3.8, module docs | grep-verified | determinism 1–3 | ✓ (RNG touched at exactly actors.rs:238, mod.rs:842 + reseeds) |

Threshold scaling (check D): `scaled()` is applied to Elroy 1/2 dots, fruit
triggers, personal and global house limits — and to nothing else (fright and
no-dot times are seconds, unscaled ✓); classic map bypasses inside `Map` ✓;
Hard routes every lookup through `effective_level` ✓; L17 `fright_ticks: None`
⇒ reversal-only ✓ (tested); levels ≥ 21 clamp to the 21+ row forever ✓.

## Re-derived test expectations (check B inventory)

Every numeric literal below re-derived from docs/map sources, agreeing with the
tests unless flagged:

- **sim_movement (≈34)**: spawn (112,212)/(12,44)/(128,60); 80% = exactly
  1 px/tick over ticks 1–8; dot tick 9 @ x=103, 1-tick stop, resume 102;
  RING pellet cadence 5/14/23/32, energizer stop 33–35, resume y=14; corner
  traces (102,211)→(100,209)→(100,208) and post-turn (101,211)/(100,210);
  reversal 108→109; wall park + anim freeze; wrap +223 (224 px) and +207
  (208 px) with column 28 preserved; Ready{50}.
- **sim_targeting (≈27)**: 14 `chase_target` hand values (all four ghosts,
  all up-overflow cases, Clyde 64/49/61 dist² boundaries); scatter corners
  (25,−1)/(2,−1); tie positions (37,44)/(12,37)/(12,51) via 75% stepping
  `t − ceil(t/16)` and J/M tie geometry (37=37, 2=2); eyes target (5,1).
- **sim_modes (≈14)**: 420/421 flip boundary; Blinky x=55@t31 (accum 16),
  fright steps 33/35/36/38/39/41/43/44, reversal at x=47 entering (5,1);
  E=113 energizer tick; fright exactly 360 ticks; 5 white groups from the
  140-tick window; frozen schedule (no flip before death > 421).
- **sim_house (≈22)**: Pinky release tick 1 + 48-tick 24-px climb to
  (112,116) Left; Inky at pellet 30 ✓ / **Clyde at 60 = F1 (doc says 90)**;
  timer releases 1/240/480; bounce 136@t8, 144@t24, 136@t40 (32-tick cycle,
  ±4 px); scale 0.15 set {Inky 5, Clyde 9; global 1/3/5} with releases
  (1,Pinky)(3,Inky)(quirk at 5)(7,Clyde via preserved 7+2=9); freeze 60;
  eyes 2 px/tick; Vertigo global Pinky = round(7·340/244) = 10.
- **sim_scoring (≈20)**: pass-through 47/48@t35 → 48/47@t36; kill @t36 at
  (48,51) same tile; proximity dx=4@t26 (no eat, exact <1024 subpx boundary)
  vs dx=3@t27 (eat, 200, chain 1) from x=13+⌊9(t−4)/8⌋ and 56−(⌊5t/8⌋−1);
  chain 200/400/800/1600; fruit trigger round(70·0.1)=7, eaten (9,100),
  popup 119; expiry 541–600 (+ seed-19 comment's 544 independently
  recomputed); hard board 1 = cherries 100, history [0].
- **sim_sequences (≈24)**: READY 258 = 138+120 with the 138 visibility
  boundary; death map: 18 pellets, Elroy-1 from start, collision t37
  (x = t+10 vs 84−t), freeze 60, anim 160, LifeLost @ 220, READY 120,
  lives 2, pellets total−2 preserved; clear tick 53
  (15+⌊9(t−16)/8⌋ = 56), 234 = 120+4·2·12+18, ghosts hidden from 120,
  history [0,1]; ghost-score freeze 60 (Pinky, 400, chain 2), prior eyes
  moving, bounce paused; progression: A.1 symbol map, extra life once on
  L11's fruit (600+8800+50+3000 = 12,450 ≥ 10,000), L16/18 fright vs L17
  reversal-only, level 25 reached, lives 4.
- **sim_elroy (≈9)**: scaled thresholds 40/20; x=68@t16, 45@t39 (1 px/tick),
  28@t55 (Elroy-2 from pellet 5 of 25: 17 px in 16 ticks at 272 subpx);
  scatter-override targets (pac tile vs (0,−1)); suspension: corner (11,−1)
  target + 15 px/16 ticks at plain 75%.

Total ≈ 145 re-derived values; **1 disagreement (F1)**, all others confirmed.

## Not checked

- Upstream accuracy of the dossier/supplement transcriptions (taken as ground
  truth per the brief), `rules` table data (already reviewed), the `map`
  parser/validator internals, render/app/replay modules, and real-terminal
  behavior. `sim_determinism.rs` was read for check E but its property
  assertions were not independently re-derived (they are invariants, not
  doc-sourced numbers).
