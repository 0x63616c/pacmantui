# Traceability matrix: reference rule/table → implementation → verified test

- **Status**: complete for the arcade rules data (`rules`), the simulation
  mechanics (`sim`), and the custom-map rule adaptations (`map`); scope
  boundary in [Coverage note](#4-coverage-note-what-is-not-traced-here).
- **As of commit**: `fa29a31` (review findings F1–F6 resolved).
- **Fulfils**: ACCEPTANCE.md E3 ("Traceability: rule→impl→independent test").

## 1. Purpose & how to read

Every gameplay rule and table value is traced from its reference source to
the code that implements it and the test that asserts it. Independent
verification was performed by the two review reports — **review-rules.md**
(215 scripted checks of every table cell and prose value, at `a2550d6`) and
**review-sim.md** (~145 test expectations re-derived by hand from the docs
alone, at `61155b0`, with resolutions confirmed for F1–F6) — this matrix
cross-links those verdicts rather than re-deriving them.

Conventions:

- **Reference** citations: "dossier §N" = `docs/research/dossier-mechanics.md`;
  "supp §N" = `docs/research/arcade-supplements.md`; "A.1"/"A.2"/"schedule" =
  `docs/research/tables.md` (JSON twin: `tables.json`); "map-format" =
  `docs/map-format.md`.
- **Impl** names file + fn/const. Sim files are `src/sim/{mod.rs,actors.rs,
  timings.rs}`; rules are `src/rules/{mod.rs,tables.rs}`; map is
  `src/map/mod.rs`; speed fixed-point lives in `src/types.rs`.
- **Test** prefixes: `rules::` = `tests/rules_tables.rs`; `movement::` =
  `tests/sim_movement.rs`; likewise `targeting::`, `modes::`, `house::`,
  `scoring::`, `sequences::`, `elroy::`, `determinism::` for the other
  `tests/sim_*.rs` files; `map_classic::` / `map_custom::` /
  `map_validation::` = `tests/map_*.rs`. Shared sim mini-maps live in
  `tests/sim_helpers.rs`; per its header, all expectations are hand-derived
  from the research docs, never from production constants.
- **Verified** names the review record: "rules §N" = a numbered subsection of
  review-rules.md "Detailed verification results" (or a finding Fn there);
  "sim audit" = the review-sim.md "Mechanics audit summary" row of that name;
  "sim Fn" = a review-sim finding + its Resolutions entry; "sim decision n" =
  its "seven interpretation decisions" item n.
- **Exactness**: *exact* = value identical to the source (where the source
  speaks seconds, under the documented ×60 s→ticks conversion, deviation D8);
  *Dn* = documented deviation/approximation n in
  [§3](#3-documented-deviations--approximations).

## 2. Matrix

### 2.1 Reference tables & prose data (`rules` module)

12 rows. All data literals were cell-diffed against `tables.json` by
review-rules.md (its Method steps 1–4; test-literal honesty audited in its §3).

| Reference | Impl | Test(s) | Verified | Exactness |
|---|---|---|---|---|
| Table A.1, 21 rows × 15 columns (speeds, Elroy dots, fright, bonus) | `tables.rs` `LEVEL_SPECS` (`row`/`secs_to_ticks`); `rules/mod.rs` `Rules::level_spec` | `rules::table_a1_every_row_normal`, `::table_a1_21_plus_row_applies_to_every_later_level`, `::table_a1_boundaries` | rules §1, §3 | exact (fright secs ×60, D8); the `~` dots-speed columns are transcribed but deliberately unused — see sim row "Speeds" |
| A.1 fright dashes on levels 17/19/20/21+ (no blue, reversal kept) | `LEVEL_SPECS` fright `Option` tuple (all-or-nothing by construction) | `rules::no_frightened_mode_levels` (level 18 as control) | rules F6 | exact |
| Speed unit: 100% = 75.75757625 px/s (A.1 title; dossier §2.1) | `types.rs` `FULL_SPEED_FIX8 = 320`, `Fix8::speed_from_percent` | exercised by every speed assertion in `movement::`/`modes::`/`elroy::` | sim Method ("exact for every table percent") | exact — 320/256 px/tick; `round(320·pct/100)` is lossless for all table percents |
| Scatter/chase schedule, 3 bands × 8 phases (tables.md body table) | `tables.rs` `SCHEDULE_LEVEL_1`/`_LEVELS_2_4`/`_LEVELS_5_PLUS`; `Rules::schedule` | `rules::schedule_all_three_bands`, `::schedule_hard_uses_effective_level` | rules Method 4 | exact (secs ×60, D8; "1/60" → 1 tick) |
| Table A.2 hard mapping (level elimination {1,3,6,19,20}) | `Rules::effective_level`, `Rules::level_spec` | `rules::effective_level_normal_is_identity`, `::effective_level_hard_every_table_a2_row`, `::effective_level_hard_beyond_table_continuation`, `::level_spec_hard_returns_effective_row` | rules §4 (S1; re-implemented for boards 1..2000) | exact; the ≥16 → +5 continuation is proven the only possible reading (rules §4) |
| A.2 prose: hard bonus symbol/points follow the *board*, not the level | `sim/mod.rs` `bonus_spec` (reads the `Normal` row) | `scoring::hard_board_one_fruit_is_cherries_100` | rules F2 (recipe verified against every A.2 row) + sim audit "Fruit" | exact per A.2 prose (D12) |
| House dot limits 0/0/30/60 → 0/0/0/50 → all 0 (dossier §3.10) | `Rules::house_dot_limits` | `rules::house_dot_limits_classic` | rules §2 | exact; Hard routing via `effective_level` is the documented interpretation (rules F3) |
| Global counter limits 7/17/32 (dossier §3.10) | `Rules::global_dot_limits` | `rules::global_dot_limits_after_death` | rules §2 | exact |
| No-dot release timer 4 s / 3 s from level 5 (dossier §3.10) | `Rules::no_dot_release_ticks` | `rules::no_dot_release_timer` | rules §2 | exact (240/180 ticks, D8) |
| Fruit triggers at 70 / 170 dots (dossier §5.2) | `Rules::fruit_trigger_dots` | `rules::fruit_triggers` | rules §2 | exact |
| Ghost chain 200/400/800/1600 (dossier §5.1) | `Rules::ghost_chain_score` | `rules::ghost_chain_scores` | rules §2 | exact |
| Extra life at 10,000 (dossier §5.3: machine default, not tabulated) | `Rules::extra_life_score` | `rules::extra_life_at_10_000` | rules §2 | DIP default, D11 |

### 2.2 Simulation mechanics (`sim` module)

24 rows: the 22 rows of review-sim.md's "Mechanics audit summary" (every
verdict ✓ there; cited without re-derivation) plus the two behaviors whose
coverage was added by the F1/F2/F6 resolutions.

| Mechanic (reference) | Impl | Test(s) | Verified | Exactness |
|---|---|---|---|---|
| Speeds: A.1 MOVEMENT columns as Fix8; `~` dots columns unused (dossier §2.1, §8.2, A.1) | `mod.rs` `pac_speed`/`ghost_speed` (+ `types.rs` `speed_from_percent`) | `movement::pac_moves_exactly_one_px_per_tick_at_80_percent`, `elroy::elroy_stages_scaled_thresholds_and_speeds` | sim audit "Speeds" | exact; dots-average speeds emerge from the eating-pause mechanic instead (audit: "no `pac_dots_speed` use anywhere in sim") |
| Eating pauses: 1 tick per dot, 3 per energizer (dossier §2.4) | `timings.rs` `DOT_EAT_PAUSE_TICKS`/`ENERGIZER_EAT_PAUSE_TICKS`; `actors.rs` `move_pac` | `movement::dot_eating_pauses_one_tick`, `::energizer_eating_pauses_three_ticks` | sim audit "Eating pauses" | exact |
| Buffered turns, cornering + 45° cut, instant reversal, wall stop (dossier §2.2–§2.3) | `actors.rs` `pac_step`/`try_turn` | `movement::buffered_turn_corners_diagonally`, `::reversal_is_instant`, `::pac_stops_at_walls_and_anim_freezes`, `::post_turn_corner_corrects_backwards` | sim audit "Buffered turns…"; sim decision 3 | window size exact; center shifted 1 px — D3 |
| Warp tunnels, both axes, modulo px (map-format "Tunnels and warping"; dossier §2.5) | `map/mod.rs` `Map::warp`, applied per px step | `movement::warp_tunnel_classic`, `::warp_tunnel_custom` | sim audit "Warp" | exact per format spec |
| Ghost decision: one-tile look-ahead, Up>Left>Down>Right tie-break, squared-distance test (dossier §3.4) | `actors.rs` `choose_dir` | `targeting::tie_break_left_over_right`, `::tie_break_up_over_down`, `::red_zone_bans_up_for_scatter_ghosts` | sim audit "Ghost decision" | exact |
| Red zones: no upward turns at the four tiles; frightened/eyes exempt (dossier §3.7; supp §12) | `actors.rs` `choose_dir`/`choose_frightened` | `targeting::red_zone_bans_up_for_scatter_ghosts`, `::frightened_ignores_red_zone`, `::eyes_ignore_red_zone` | sim audit "Red zones" | exact |
| Chase targets per ghost incl. Pinky/Inky up-overflow bug, Clyde 8-tile switch (dossier §3.6) | `actors.rs` `chase_target` | `targeting::blinky_targets_pac_tile`, `::pinky_four_ahead_with_up_overflow`, `::inky_pivot_and_doubled_vector_with_up_overflow`, `::clyde_eight_tile_switch`, `::scatter_then_chase_targets_on_classic`, `::blinky_and_clyde_ignore_pac_direction` | sim audit "Per-ghost targets" | exact |
| Scatter corners & eyes target tile (dossier §3.5; supp §8 resolves §8.1's one-row question) | `maps/*.pmtoml` scatter targets + `actors.rs` `target_tile` | `targeting::scatter_then_chase_targets_on_classic`, `::eyes_target_is_house_door_tile`; map data: `map_classic::scatter_targets_dossier_measured` | sim audit "Scatter corners" | exact |
| Scatter/chase schedule, flip reversals on next tile entry, timer frozen in fright (dossier §3.2–§3.3) | `mod.rs` `advance_mode_timers`/`queue_reversals` | `modes::scatter_one_flips_exactly_at_tick_421`, `::no_flip_at_tick_420`, `::energizer_reverses_ghosts_at_next_tile_entry`, `::fright_360_ticks_5_flashes_and_frozen_schedule` | sim audit "Schedule" ("420/421 boundary exact") | exact |
| Frightened: A.1 duration & flash count, clockwise-retry random turn (dossier §3.8; A.1) | `mod.rs` `energize`/`fright_is_white`; `actors.rs` `choose_frightened` | `modes::energizer_reverses_ghosts_at_next_tile_entry`, `::fright_360_ticks_5_flashes_and_frozen_schedule`, `::second_energizer_restarts_fright` | sim audit "Frightened"; sim decision 6 | duration/counts exact; flash half-period — D6 |
| House personal dot counters, preference order, released-counter deactivation (dossier §3.10) | `mod.rs` `on_pellet_counters`/`first_housed`/`evaluate_releases` | `house::pinky_releases_immediately_and_exits_in_48_ticks`, `::classic_level1_inky_at_30_dots_clyde_at_90` | sim audit "House personal counters" (impl ✓) + sim F1 Resolution (test rebuilt: Clyde correctly at pellet 90, counter-path proven via same-tick `ate` correlation) | exact |
| Global counter after death, releases at 7/17/32, Clyde-at-32 deactivation quirk (dossier §3.10) | `mod.rs` `on_pellet_counters` (equality-on-increment) | `house::global_counter_7_17_32_with_clyde_deactivation_quirk`, `::vertigo_scales_global_counter_thresholds` | sim audit "Global counter"; sim decision 4 | exact on classic; scaled limits on custom maps — D4/table 2.3 |
| No-dot force release 240/180 ticks (dossier §3.10) | `mod.rs` `tick_no_dot_timer` | `house::no_dot_timer_force_releases_every_240_ticks` | sim audit "No-dot force release" | exact (D8) |
| House bounce/leave/enter speeds and exit point (supp §8/§10) | `actors.rs` `house_bounce`/`house_leave`/`house_enter`/`house_exit_point`; `timings.rs` `HOUSE_SPEED_PCT`/`HOUSE_BOUNCE_PX` | `house::pinky_releases_immediately_and_exits_in_48_ticks`, `::house_bounce_amplitude_and_period`, `::eyes_full_round_trip` | sim audit "Bounce/leave/enter" | exact (SUPPLEMENT-tagged constants) |
| Mode change while housed ⇒ ghost exits facing Right (dossier §3.9) | `mod.rs` `queue_reversals` sets `exit_right`; consumed in `actors.rs` (`activate_ghost`) | `house::mode_flip_while_housed_makes_ghost_exit_right` | sim F6 Resolution (gap closed post-review) | exact |
| Outward door transit continues during the death freeze; paused during ghost-eaten pause (supp §10) | `mod.rs` `sequence_tick`, `DeathFreeze` arm runs `house_leave` for `Leaving` ghosts | `house::leaving_ghost_finishes_door_transit_during_death_freeze` | sim F2 + Resolution (deviation found, then fixed) | exact (post-fix) |
| Eyes at 2 px/tick, revival in house, Blinky re-exits (supp §2/§8) | `actors.rs` `move_eyes`/`house_enter`; `timings.rs` `EYES_SPEED_PCT` | `house::eyes_full_round_trip` | sim audit "Eyes" | exact |
| Cruise Elroy stages 1/2, scatter override, suspension until Clyde leaves (dossier §3.11) | `mod.rs` `elroy_stage`; `actors.rs` `target_tile` | `elroy::elroy_stages_scaled_thresholds_and_speeds`, `::elroy_chases_during_scatter`, `::elroy_suspended_after_death`; `sequences::death_sequence_timing_and_pellet_preservation` | sim audit "Elroy"; sim decision 7 | exact; dot thresholds scale on custom maps — table 2.3 |
| Collision: shared-tile death, pass-through bug preserved, <4 px proximity eat when frightened (dossier §4.1; supp §12) | `mod.rs` `check_collisions` (post-movement, once per tick) | `scoring::pass_through_tile_swap_is_survived`, `::same_tile_collision_kills`, `::frightened_ghost_eaten_by_proximity` | sim audit "Collision" ("boundary dx=4 exact") | exact; two uncited micro-conventions — D13 |
| Ghost chain 200→1600, reset per energizer/level (dossier §5.1; supp §12) | `mod.rs` `eat_ghost`/`energize` | `scoring::ghost_chain_200_400_800_1600` | sim audit "Ghost chain" | exact |
| Fruit: spawn triggers, eat, 541–600-tick expiry, score popup (dossier §5.2; supp §9/§11; A.2 prose) | `mod.rs` `spawn_fruit`/`update_fruit`/`bonus_spec`; `timings.rs` `FRUIT_TICKS_MIN/MAX`, `POPUP_TICKS` | `scoring::fruit_spawns_on_trigger_and_is_eaten`, `::fruit_expires_within_the_rom_window`, `::hard_board_one_fruit_is_cherries_100` | sim audit "Fruit"; sim decision 2 (seed-19 draw independently recomputed) | window exact; uniform draw — D2; popup 120 f — D14 |
| Sequences: READY 258/120, death 60+160, level-clear 234, ghost-score freeze 60 (supp §1/§3/§4/§5/§13) | `mod.rs` `sequence_tick`; `timings.rs` `READY_*`, `DEATH_*`, `LEVEL_CLEAR_*`, `GHOST_SCORE_FREEZE_TICKS` | `sequences::first_ready_is_two_phase_258_ticks`, `::death_sequence_timing_and_pellet_preservation`, `::level_clear_flash_timing_and_reset`, `::ghost_score_freeze_semantics` | sim audit "Sequences"; sim decision 1 | exact phase-locked ROM values (supp §0.1/§13) |
| Progression: A.1 symbol sequence, 7-slot fruit history, extra life once, L17 reversal-only, 21+ forever (dossier §5.2–§5.4; A.1) | `mod.rs` `start_level`/`start_next_level`/`housekeeping`; `FRUIT_HISTORY_LEN` | `sequences::level_clear_flash_timing_and_reset`, `::progression_fruit_history_extra_life_and_level_17` | sim audit "Progression" | exact; continuation past 255 — D9 |
| Determinism: no map iteration, integer math, exactly 2 RNG consumption points, reseed per level/death (dossier §3.8; sim module docs) | `mod.rs` `Rng` + reseeds in `start_level`/`respawn_after_death`; draws in `actors.rs` `choose_frightened` and `mod.rs` `spawn_fruit` | `determinism::same_seed_same_inputs_identical_run`, `::property_20k_random_ticks_classic`, `::property_20k_random_ticks_custom` | sim audit "Determinism" (consumption points grep-verified) | reseed rule exact; fruit draw is D2 |

### 2.3 Custom-map rule adaptations (`map` module)

5 rows. These are engine adaptations by definition (no arcade source exists
for non-244-pellet mazes); the *rule* being traced is map-format.md "Rule
adaptations (non-244-pellet maps)" (binding per IMPLEMENTATION.md "Key
technical decisions"). The sim-side application was independently verified by
review-sim.md's "Threshold scaling (check D)" paragraph; the map parser
itself has no independent review (see §4).

| Rule (map-format "Rule adaptations") | Impl | Test(s) | Verified | Exactness |
|---|---|---|---|---|
| `scaled = round(classic × pellets_total / 244)`, min 0, round-half-up integer arithmetic | `map/mod.rs` `Map::scale_dot_threshold` | `map_custom::threshold_scaling_uses_pellet_ratio`, `map_validation::default_scaling_uses_pellet_ratio` | sim check D (application); map tests only for the formula | adaptation (documented, binding) |
| Classic map bypasses scaling entirely | `Map::is_classic` short-circuit inside `scale_dot_threshold` | `map_classic::classic_is_exempt_from_threshold_scaling` | sim check D ("classic map bypasses inside `Map` ✓") | exactness-preserving |
| Optional `[rules] threshold_scale` override (finite, > 0) | `Map::parse` + `scale_dot_threshold` | `map_validation::threshold_scale_override_applies`, `::threshold_scale_must_be_positive` | map tests only | adaptation |
| Scaling applies to Elroy 1/2 dots, fruit triggers, personal **and global** house limits — and to nothing else | `sim/mod.rs` `scaled()` call sites | `elroy::elroy_stages_scaled_thresholds_and_speeds` (40/20), `house::vertigo_scales_global_counter_thresholds` (round(7·340/244)=10), `scoring::fruit_spawns_on_trigger_and_is_eaten` (round(70·0.1)=7) | sim check D + sim decision 4 | adaptation; global-limit scaling is an extrapolation sanctioned by IMPLEMENTATION.md's "counters" clause |
| Time-based values (fright seconds, no-dot timer) are never scaled | absence of `scaled()` on those paths | asserted indirectly by the tick-exact fright/no-dot tests on custom mini-maps | sim check D ("fright and no-dot times are seconds, unscaled ✓") | exact |

## 3. Documented deviations & approximations

Consolidated ledger; nothing here is new analysis — each entry points at
where the deviation is documented and which review judged it. D1–D7 are the
seven interpretation decisions of review-sim.md, "all judged sound".

| # | Deviation / approximation | Documented at | Sanctioned by |
|---|---|---|---|
| D1 | GhostScoreFreeze composite semantics (what freezes vs keeps running during the ghost-eaten pause) | supp §1; `sequence_tick` | sim decision 1; test `sequences::ghost_score_freeze_semantics` |
| D2 | Fruit display duration drawn uniformly from 541–600 ticks via the sim RNG instead of emulating the global 60-frame clock; distribution-equivalent, but durations repeat within a level because the RNG reseeds (disclosed) | supp §9 "Implementation"; `timings.rs` `FRUIT_TICKS_MIN/MAX` doc; sim module-doc deviation list | sim decision 2 (seed-fold + xorshift independently re-implemented) |
| D3 | Cornering window measured as 4 px pre / 3 px post instead of the dossier's 3/4 — a real 1-px shift caused by this engine's tile centers at pixel offset 4 (`types::PxPos::tile_center`) vs the arcade's 3; window size and 45° double-speed cut preserved | dossier §2.2; sim module-doc deviation list | sim decision 3 ("correctly self-documented") |
| D4 | Global house counter: equality-on-increment check at exactly 7/17/32 (reproduces the dossier's keep-them-housed trick), and global limits scaled on custom maps (an extrapolation) | dossier §3.10; IMPLEMENTATION.md "counters" clause; map-format "Rule adaptations" | sim decision 4; `house::vertigo_scales_global_counter_thresholds` |
| D5 | House releases staggered at most one ghost per tick (ROM checks all three each frame — gameplay-negligible); dead-end reversal fallback (unreachable on classic maze); `reverse_pending` discarded on eyes conversion | sim module-doc deviation list; supp §12 | sim decision 5 |
| D6 | Frightened flash half-period: 14 ticks is an APPROXIMATION (ROM rate `#0AC3` not extracted; common emulation value), shrunk when a level's fright time cannot fit the flash count so Table A.1's "# of Flashes" is always honored exactly | `timings.rs` `FRIGHT_FLASH_HALF_TICKS`; `mod.rs` `fright_is_white` | sim decision 6 ("best-supported reading") |
| D7 | Elroy suspension lifts precisely on Clyde's InHouse→Leaving transition; immediate on maps that spawn Clyde outside | dossier §3.11; `release_ghost` | sim decision 7 |
| D8 | All table "seconds" converted at a flat 60 ticks/second (one sim tick = one arcade frame at 60.606061 Hz), so long phases deviate ~1% from arcade wall-clock; sanctioned by dossier §6, though the code comments cite §8.3, whose own resolution leans the other way | `rules/mod.rs` + `tables.rs` doc comments; dossier §8.3 vs §6 | rules F1 (citation direction flagged, choice judged defensible); tick literals in tests script-verified against source seconds |
| D9 | Level 256: `u32` level counter continues on the A.1 "21+" row forever instead of reproducing the kill screen | dossier §6 "Level 256" (which itself recommends safe continuation); sim module-doc deviation list; IMPLEMENTATION.md; ACCEPTANCE.md G9 | deliberate per IMPLEMENTATION.md; exercised to level 25 by `sequences::progression_fruit_history_extra_life_and_level_17` |
| D10 | Sprite art is original, authored "in the arcade *style*" — inspired, not pixel-identical; no ROM bitmap data copied | `src/render/sprites.rs` module doc; IMPLEMENTATION.md W1-RENDER ("original sprite art") | outside table traceability; checked visually per ACCEPTANCE.md R8 |
| D11 | Extra life at 10,000 points and 3 starting lives are machine DIP-switch defaults the dossier does not tabulate | dossier §5.3; supp §6 (ROM decode `#26D0`, bonus table `0x2728`); `Rules::extra_life_score` doc; `timings.rs` `STARTING_LIVES` | rules §2 ("honestly flags it as a DIP-switch default") |
| D12 | Hard-mode fruit symbol/score read from the Normal row so the displayed (lagged) symbol governs points, per A.2 prose — the pure-gameplay `level_spec(_, Hard)` row must not be used for bonus fields | `sim/mod.rs` `bonus_spec` doc; tables.md A.2 prose | rules F2 (workaround verified against every A.2 row); `scoring::hard_board_one_fruit_is_cherries_100` |
| D13 | Two uncited collision micro-conventions (no source exists to cite): fruit eaten by tile equality with `fruit_pos().tile()` (eat point asymmetric up to 8 px by approach side for the boundary-straddling classic fruit); fixed ghost-index order decides same-tile mixed frightened/deadly coincidences | review-sim F5 | sim F5 ("benign, consistent with §4.1's tile conventions; flagged for the record; no action") |
| D14 | Remaining cosmetic APPROXIMATION-tagged constants: fruit-score popup 120 f (supp §11, "~2 s" not byte-verified), energizer blink 10 f, Pac-Man chomp 2 px/phase — `timings.rs` is the single home for every non-dossier constant, each with a DOSSIER / SUPPLEMENT / APPROXIMATION provenance tag | `src/sim/timings.rs` per-constant docs | sim review read every constant; none affects gameplay state |
| D15 | Tunnel side-pocket wall art and outer border: the generic contour wall renderer draws single-outline closed shapes with pocket mouths closed ~2 px inside the screen edge and the border inset ~2 px (arcade: double-outline blocks open to the edge). Cosmetic only — maze layout, topology, pellets, and collision are tile-exact (04-fidelity.md). | `src/render/scenes.rs` (generic contour renderer); found by docs/validation/04-fidelity.md F-VIS-1 | accepted as part of the original-art scope alongside D10 |

## 4. Coverage note: what is NOT traced here

- **Render, app/menu, replay, persistence** (`src/render/**`, `src/app/**`,
  `src/main.rs`, `src/replay/**`): no arcade reference table governs them, so
  they are validated by live acceptance evidence in `docs/validation/`
  (kitty smoke test, live-play replays and screenshots) against
  ACCEPTANCE.md rows R1–R8/A1–A3/E4, plus their own engineering tests
  (`tests/render_*.rs`, `tests/app_*.rs`, `tests/replay_format.rs`) — which
  assert code behavior, not source-document values.
- **Map parser/validator internals**: tested against `docs/map-format.md` and
  the dossier maze transcription (`map_classic::pellet_counts_match_dossier`,
  `::spawns_are_pixel_exact`, `::scatter_targets_dossier_measured`, the
  `map_validation::` corpus), but no independent review covered the parser —
  review-sim.md "Not checked" excludes it (that review did read the consumer
  APIs and both shipped `.pmtoml` maps).
- **Upstream transcription accuracy**: both reviews treat
  `docs/research/tables.md`/`tables.json` and the dossier/supplement
  transcriptions as ground truth. The transcription's own PDF↔HTML
  cell-by-cell cross-check is recorded in tables.md "Verification notes"
  (no genuine data conflicts).
- **Residual acknowledged test gaps** (sim F6, no wrong behavior found): the
  §3.10 global-counter stays-active-forever branch, the flash-shrink path on
  1-second fright levels, and Hard schedules driven through the sim beyond
  the fruit test — all guarded at the `rules` level or by code reading, per
  the review. The §3.9 exit-right gap from the same finding is now closed
  (table 2.2).
- **`sim_determinism` property assertions** are invariants, not doc-sourced
  numbers; review-sim read them for its determinism check but did not
  re-derive them (its "Not checked" section).
