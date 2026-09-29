# Independent Review: `rules` module vs reference documentation

- **Reviewer**: independent (did not author any of this code)
- **Date**: 2026-09-28
- **Scope**: `src/rules/tables.rs`, `src/rules/mod.rs`, `tests/rules_tables.rs` vs
  `docs/research/tables.json` (verified transcription), `docs/research/tables.md`,
  `docs/research/dossier-mechanics.md`
- **Commit under review**: working tree at `a2550d6` (rules code from `14b37bf`)

## Verdict: APPROVED-WITH-NOTES

No data errors found. Every cell of every table in the implementation and in the test
literals matches the verified transcription, the A.2 hard mapping is provably the
"Nth surviving level" function, and all 16 tests pass. Two minor documentation/API-contract
observations and a handful of small coverage gaps are recorded below; none blocks.

## Method

Scripted comparison, not eyeballing. One throwaway script
(`scratchpad/verify_rules.py`, kept outside the repo) performing **215 individual
checks**, all passing:

1. Parsed the 21 `row(...)` lines of `LEVEL_SPECS` in `src/rules/tables.rs` (labels,
   symbol, 9 numeric columns, 5-tuple fright `Option`) and compared each cell against
   `tables.json → table_a1.rows` (21 rows × 15 data columns).
2. Parsed the 21 literal tuples of `const TABLE_A1` in `tests/rules_tables.rs` and
   compared each cell against `tables.json`, with fright ticks required to equal
   `fright_time_seconds × 60`.
3. Independently re-parsed the Markdown Table A.1 in `tables.md` (16-column rows,
   `~`/`%` stripped, `–` → null) and diffed it against `tables.json` — the two
   reference artifacts agree cell-for-cell.
4. Parsed all three schedule statics in `tables.rs` (via `scatter_secs`/`chase_secs`
   arguments, `SCATTER_ONE_TICK`, `CHASE_INDEFINITE`) and the three test `const`
   schedules, and compared both against `tables.json → scatter_chase_schedule`
   (seconds × 60; `1/60` → 1 tick; `indefinite` → `None`). Also diffed the Markdown
   schedule table against the JSON.
5. Re-implemented `effective_level(_, Hard)` in Python and compared it for boards
   1..2000 against ground truth defined independently from A.2: "the Nth normal level
   not in {1, 3, 6, 19, 20}". Verified the A.2 hard-column sequence
   `[2,4,5,7,…,18,21+]`, the eliminated set, the test's `hard_sequence` literal, and
   the bonus-ordinal justification (see finding S1).
6. Cross-checked `tables.json → speed_summary` bands against A.1 (consistent;
   A.1's dashed fright cells at 17/19/20 sit inside the summary's "5–20" band, which
   the summary shows non-dashed — a known coarsening of the source's own summary
   table, not a transcription issue).
7. `cargo test --test rules_tables` run locally: **16 passed; 0 failed** (0.00 s).

Prose-sourced values were checked by reading the cited sections of
`dossier-mechanics.md` directly (quotes below).

## Findings

| # | Severity | Location | Expected vs actual | Source citation |
|---|----------|----------|--------------------|-----------------|
| F1 | minor | `src/rules/mod.rs:14-19`, `src/rules/tables.rs:17-23` (doc comments) | Comments cite "mechanics doc §8.3" in support of the flat 60-ticks-per-table-second reading. §8.3's own stated **Resolution** recommends the opposite: "treat every 'second' as a count of frames at the real 60.606061 fps **for authenticity** … using flat 60 fps changes phase lengths by ~1%". The flat-60 choice is actually sanctioned by §6 ("or, pragmatically, 60 fps — the dossier itself conflates the two"). The choice itself is defensible and thoroughly documented; only the citation direction is misleading. Practical effect: long phases deviate ~1% from arcade wall-clock (the 1033 s chase ≈ 10 s shorter at a 60 Hz sim). | dossier-mechanics.md §8.3 (lines 628-636), §6 (lines 549-552) |
| F2 | minor | `src/rules/mod.rs:125-138` (`level_spec` contract) | In hard mode the arcade displays the **lagged** bonus symbol and "bonus point values follow the displayed symbol, not the level" (tables.md A.2 prose). `level_spec(level, Hard)` returns the pure gameplay row (e.g. hard board 1 → Strawberry/300, arcade shows Cherries/100). The doc comment gives the correct recipe — read the bonus fields of `level_spec(level, Difficulty::Normal)` — and I verified by script that this recipe reproduces the A.2 Hard-Bonus column exactly for every tabulated board. But nothing in the API or tests enforces it; a sim that naively awards `level_spec(level, Hard).bonus_points` is silently non-arcade-accurate in hard mode. Recommend the sim reviewer check the call site when fruit scoring lands. | tables.md lines 96-99: "The bonus-symbol sequence is *not* eliminated, so symbols lag behind the true level (e.g. hard's first board is really level 2 gameplay but displays cherries, and bonus point values follow the displayed symbol, not the level)." |
| F3 | note | `src/rules/mod.rs:159-185` (`house_dot_limits`, `no_dot_release_ticks` apply `effective_level` under Hard) | The dossier gives these values per gameplay level (§3.10) and never mentions hard mode for them; routing through `effective_level` is an interpretation, but the correct one given A.2's prose that level elimination is "the only gameplay difference" — hard board 1 is level-2 gameplay in every respect. Tested (`house_dot_limits_classic`, `no_dot_release_timer` hard cases). | tables.md lines 95-96: "The only gameplay difference is that five levels — 1, 3, 6, 19, and 20 — are eliminated from play."; dossier-mechanics.md §3.10 |
| F4 | note | `tests/rules_tables.rs` coverage | Defensive clamps untested: `effective_level(0, _)` (clamp to 1, `mod.rs:112`), `ghost_chain_score(0)` (→200) and `ghost_chain_score(5..)` (→1600) (`mod.rs:199-206`), `level_spec(0, _)`. Also `schedule(_, Hard)` beyond board 3 untested (trivially the 5+ band). All are documented defensive paths, not table data. | — |
| F5 | note | `tests/rules_tables.rs:319-339` | Full `LevelSpec` equality under Hard is asserted directly only for boards 1, 16, and `u32::MAX`. Boards 2–15 are covered *compositionally* (`effective_level_hard_every_table_a2_row` proves the mapping; `table_a1_every_row_normal` proves each row) — valid because `level_spec` is implemented exactly as `LEVEL_SPECS[effective − 1]`, but a refactor decoupling them would not be caught for those boards. Acceptable. | — |
| F6 | note | `src/rules/tables.rs:47`, fright encoding | The five fright columns are encoded as a single `Option` tuple, enforcing the source's all-or-nothing dashing. Script-confirmed against tables.json: fright columns are null exactly and only together, exactly on rows 17, 19, 20, 21+. The "energizers still reverse them" caveat matches tables.md ("they no longer turn blue at all (they still reverse direction, however)", line 47) — behavioral enforcement is the sim's job, correctly out of scope here. | tables.md lines 45-48 |

## Detailed verification results

### 1. Table A.1 (`tables.rs` vs tables.json) — clean

All 21 rows × 15 data columns match, including the traps: level 3 Elroy1 = 40 (not
30), level 6 fright jumps back to 5 s, level 9 = 1 s/3 flashes but level 10 = 5 s/5
flashes, level 14 = 3 s/5 flashes, level 17 dashed but 18 not, and the 21+ row's
deliberate Pac-Man 90%/~79% drop with ghosts held at 95%. `secs_to_ticks` is exact
(all source durations integral seconds). The `row()` builder assigns positionally to
same-named fields — verified by reading `tables.rs:36-82`, no transposition.

### 2. Prose-sourced values (`mod.rs` vs dossier-mechanics.md) — clean

- **House dot limits** `[0,0,30,60]` / `[0,0,0,50]` / `[0,0,0,0]` — §3.10 (lines
  423-424): "Dot limits: **Pinky always 0** (leaves immediately every level).
  **Level 1: Inky 30, Clyde 60. Level 2: Inky 0, Clyde 50. Level 3+: all 0**."
  Blinky = 0 per line 413: "Blinky is never subject to it."
- **Global limits** `[0,7,17,32]` — §3.10 (lines 429-430): "Releases: **Pinky at 7,
  Inky at 17**; and when it reaches **32**: *if Clyde is inside the house at that
  moment*, the global counter is **reset to zero and deactivated**…" The
  deactivation quirk is explicitly delegated to the sim in the doc comment — correct
  layering.
- **No-dot timer** 240/180 ticks — §3.10 (lines 441-443): "Limit: **4 seconds on
  levels 1–4, 3 seconds from level 5** ('The game begins with an initial timer limit
  of four seconds, but lowers to it to three seconds starting with level five.')."
- **Fruit triggers** `[70,170]` — §5.2 (lines 516-517): "first when **70 dots** have
  been cleared, second at **170 dots**."
- **Ghost chain** 200/400/800/1600 — §5.1 table (line 507): "Ghosts, per energizer
  chain | 200 / 400 / 800 / 1600."
- **Extra life** 10,000 — §5.3 (lines 528-530): "**Not documented in the dossier**…
  Implement the machine default of one extra life at **10,000 points**." The doc
  comment honestly flags it as a DIP-switch default the dossier does not tabulate.

### 3. Test honesty audit — clean

- **(a) Literals, not derived**: `tests/rules_tables.rs` imports only
  `pacmantui::rules::{BonusSymbol, Rules, SchedulePhase}` and
  `pacmantui::types::{Difficulty, Mode}`. The statics physically cannot leak:
  `mod tables;` is private inside `rules` and the statics are `pub(super)`
  (`tables.rs:98,151,164,177`) — compiler-enforced, not just convention. No helper
  indirection touches production data; `assert_spec_matches` consumes only the
  hand-written `const TABLE_A1`. The one derived convention is seconds → ticks
  (× 60), which is unavoidable (docs speak seconds, API speaks ticks), is declared
  in the test header, and every resulting tick literal was script-verified against
  `tables.json` seconds.
- **(b) Coverage**: complete for the data. Every A.1 row asserted on every column
  except the `Level` lookup key itself (`table_a1_every_row_normal`); 21+
  continuation at 21/22/30/100/255/1e6/`u32::MAX`; boundaries 4→5 and 20→21
  cell-named; fright-None rows 17/19/20/21+ (all five fields) with level 18 as the
  in-between control; all 3 schedule bands × 8 phases with band-boundary levels 1/2,
  4/5 and the 5+ band out to `u32::MAX`; hard schedule via boards 1-3; every A.2 row
  via the 16-board sequence plus an eliminated-level exclusion sweep over boards
  1..60; beyond-table continuation (+5) at 16/17/18/20/100 and `u32::MAX`
  saturation; all prose values including hard-mode variants. Gaps: F4/F5 only.
- **(c) Literal correctness**: scripted diff of all 21 test rows, 3 test schedules,
  and the `hard_sequence` literal against tables.json — zero mismatches.

### 4. Semantics: A.2 / `effective_level` — correct, justification verified (S1)

tables.md (lines 92-99) describes A.2 as level *elimination*: hard play simply skips
levels 1, 3, 6, 19, 20, so hard's first board "is really level 2 gameplay". Hence
hard board N = Nth surviving level, exactly what `effective_level` computes: my
independent re-implementation of "Nth normal level ∉ {1,3,6,19,20}" agrees with the
code's piecewise form (1→2, 2→4, 3→5, 4..15→+3, ≥16→+5) for every board 1..2000. So
yes — hard level 1 really plays as normal level 2 (speeds 90/85%, Strawberry row,
5 s fright), as `level_spec_hard_returns_effective_row` asserts.

The code comment's bonus-ordinal justification (`mod.rs:101-107`) checks out against
the actual A.2 columns: normal board 16's bonus is "Key 4" (tables.md line 129) and
A.2's hard 21+ row shows "Key 4" (line 134), confirming hard 21+ is board 16;
hard board 1 shows Cherries = symbol #1 (line 115). Script-verified stronger form:
for **every** tabulated hard board N, the Hard-Bonus cell equals normal board N's
symbol. The ≥16 → +5 continuation is the only possible reading (all five eliminated
levels lie at or below 20, and every such level hits the A.1 21+ row regardless), and
16→21 reproduces the table's jump from 18 straight to 21+.

### 5. Test run

`cargo test --test rules_tables`: **16 passed, 0 failed, 0 ignored** (compile +
run clean, no warnings surfaced).

## What I did NOT check

- **Upstream transcription accuracy**: I treated `tables.json`/`tables.md` as the
  verified ground truth per the review brief. I did not re-fetch the Pac-Man Dossier
  HTML/PDF; if the transcription itself were wrong, this review would not catch it
  (mitigated slightly by my independent md↔json cross-diff, which found the two
  artifacts mutually consistent).
- **Behavioral semantics owned by the sim**: energizer reversal on fright-None
  levels, scatter/chase timer pause during fright and reset on death/level-complete,
  the global-counter Clyde-at-32 deactivation quirk, Elroy suspension, eating-pause
  frames (the `~` columns' contract). `rules` only exposes data; I verified the doc
  comments assign these correctly to the sim, not that any sim implements them.
- **`types.rs` speed conversion** (`FULL_SPEED_FIX8`, percent→fixed rounding) and
  every module outside `rules` (map, sim, render, app, replay) and their tests.
- **Conformance to docs/plan/ARCHITECTURE.md contracts** (API shape ownership),
  clippy/fmt, and the full `cargo test` suite (only `--test rules_tables` was run,
  per the brief).
- **Hard-mode fruit *display* behavior end to end** (F2): I verified the documented
  workaround is mathematically correct against A.2, not that any caller uses it.
