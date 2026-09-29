# Implementation plan

Crate layout: lib (`src/lib.rs`, modules below) + bin (`src/main.rs`) so
integration tests in `tests/` use the lib. `src/bin/proto.rs` remains as the
rendering validation tool.

The compiling skeleton (types + stub signatures, committed before delegation)
IS the interface contract. Agents implement behind existing signatures; the
lead owns signature changes and integration.

## Work breakdown & ownership (no overlapping files)

| Work item | Owner | Files | Depends on |
|---|---|---|---|
| W1-RULES: tables → static data, level/difficulty resolver, schedule | agent | `src/rules/**`, `tests/rules_*.rs` | tables.json/md |
| W1-MAP: map format spec, parser, validator, classic + custom maps | agent | `src/map/**`, `docs/map-format.md`, `maps/*`, `tests/map_*.rs` | mechanics doc (maze) |
| W1-RENDER: engine from proto (terminal guard, kitty writer, compositor, sprite atlas, original sprite art, scenes/HUD) | agent | `src/render/**` | rendering.md, proto |
| W1-SIM: movement, ghosts, modes, house, collisions, scoring, fruit, death, progression | agent | `src/sim/**`, `tests/sim_*.rs` | mechanics doc, skeleton APIs |
| W2-APP: state machine, menus, persistence, fixed-timestep loop, difficulty select | agent | `src/app/**`, `src/main.rs` | W1 all |
| W2-REPLAY: record/replay + scenario tooling | agent | `src/replay/**`, `tests/replay_*.rs` | W1-SIM |
| W2-REVIEW: independent verification of tables/mechanics/tests vs sources | separate agent | review report in docs/plan/ | W1-RULES, W1-SIM tests |
| W3: integration, real-terminal acceptance, fidelity screenshots, fixes | lead | — | all |

## Key technical decisions (binding)

- Positions: integer pixels with 8-bit subpixel accumulator (`Fix8` = i32,
  1/256 px). Speeds from tables as percent of 1.25 px/tick (100% = 75.757576
  px/s at 60.606061 Hz ⇒ exactly 320/256 px/tick at 100%; percent applied in
  fixed point). One sim tick = one arcade frame.
- Tie-break order everywhere: Up > Left > Down > Right (dossier).
- PRNG: 32-bit xorshift with documented seeding; frightened turns are its only
  consumer in classic rules. Replays store the seed.
- Timings the dossier lacks (death anim, READY!, eaten pause, eyes speed…) live
  ONLY in `sim::timings` with per-constant provenance comments, filled from
  docs/research/arcade-supplements.md (research in flight; placeholders marked
  `// PROVISIONAL`).
- Map format: TOML header (metadata, spawns, scatter targets, tunnels, house,
  fruit, per-map rule adaptations) + ASCII grid section. Version field required.
  Full spec in docs/map-format.md (W1-MAP writes it; lead reviews).
- Difficulty: `Normal` uses Table A.1 level→row directly; `Hard` maps via
  Table A.2 (level elimination), no ad-hoc multipliers.
- Custom-map threshold adaptation: dot-count-dependent values (Elroy, fruit
  triggers, counters) scale by ratio of map dot count to 244, rounded to
  nearest, min 0; documented in map-format.md; classic map bypasses scaling.

## Milestones

M1 skeleton compiles (lead) → M2 wave-1 modules + unit tests green →
M3 sim integration: playable classic level w/ renderer (lead) → M4 app flow,
persistence, replay, custom map → M5 independent review + full test matrix →
M6 real-terminal acceptance + evidence → M7 README/docs/handoff.
