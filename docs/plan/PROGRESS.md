# pacmantui — progress & operating log

Goal: complete, polished Pac-Man in Rust with real pixel graphics inside Ghostty
via the Kitty Graphics Protocol. Faithful to The Pac-Man Dossier. Data-driven maps.
Repo: github.com/0x63616c/pacmantui (public — user's explicit choice, for free CI).

## Environment (verified 2026-09-28)

- Terminal: Ghostty 1.3.2 embedded in cmux.app (TERM_PROGRAM=ghostty), macOS (Darwin 27).
- This session runs in cmux workspace:1 / pane:1 / surface:1.
- Neighbor pane for playtesting: `cmux new-pane --direction right --workspace workspace:1 --focus false`.
- Terminal text capture: `cmux read-screen --surface surface:N` (text only — NOT graphics proof).
- Keyboard input to pane: `cmux send` / `cmux send-key`.
- Pixel screenshots: cmux computer-use (cmux-cua MCP tools) or macOS screencapture — to be validated.
- GitHub: authenticated as 0x63616c; repo pacmantui does not exist yet (to create, private).
- Rust 1.94.0 stable, cargo 1.94.0.

## Phase status

- [x] Environment inspection
- [x] Research: rendering → docs/research/rendering.md
- [x] Research: dossier mechanics → docs/research/dossier-mechanics.md
- [x] Research: reference tables A.1/A.2 → docs/research/tables.md + tables.json
- [x] Research: arcade supplemental timings → docs/research/arcade-supplements.md
- [x] Rendering prototype validated in real Ghostty/cmux pane (docs/validation/01-proto.md)
- [x] Implementation plan + architecture + acceptance matrix (docs/plan/)
- [x] Module contracts: compiling skeleton committed (types.rs + stubs)
- [x] W1-RULES: src/rules + table tests (independent review APPROVED — review-rules.md)
- [x] W1-MAP: map format/parser/validator + classic & Vertigo maps + docs/map-format.md
- [x] W1-RENDER: kitty renderer, sprites, HUD, menus (+ render_demo, evidence 02)
- [x] W1-SIM: full arcade rules engine → src/sim, tests/sim_* (61155b0)
- [x] W2-APP/REPLAY (lead): app state machine, menus glue, persistence, fixed-timestep
      loop, CLI (--map/--replay/--record), replay text format + tests (app awaits sim)
- [x] W2-REVIEW: independent sim/mechanics review (docs/plan/review-sim.md; F1–F6
      resolved at fa29a31)
- [x] Test suite incl. table traceability matrix (docs/plan/TRACEABILITY.md); CI green
- [x] Real-terminal playtest + visual acceptance (screenshots in docs/validation/)
- [x] Final push, README, handoff

## Final status (2026-09-29) — COMPLETE / VALIDATED

- Review findings F1–F6 resolved at `fa29a31`; 222 tests green, CI green.
- The three committed replays re-verified headlessly at that commit —
  byte-identical outcomes (classic 4600/4558t, vertigo 6360/5743t).
- Live acceptance run complete: docs/validation/03-live-gameplay.md,
  figures 10–24 (fruit, fright chain + eyes, level clears on both maps,
  pause/resize/restart/respawn, menu return, white-flash phase), plus the
  real-keys menu-tree walkthrough recorded in §Checks without figures.
- Arcade-fidelity comparison complete: docs/validation/04-fidelity.md —
  maze/pellets/start positions tile-exact, palette exact; the one cosmetic
  wall-art finding (F-VIS-1) recorded as deviation D15.
- docs/plan/TRACEABILITY.md added (rule→impl→independent test, complete for
  rules/sim/map; deviation ledger D1–D15).
- ACCEPTANCE.md matrix updated with the live-run evidence.

## Operational notes for real-terminal work

- Fresh-pixel screenshots: `docs/validation/snap_cmux.sh out.png` (focus-flip;
  occluded panes give stale pixels otherwise).
- `cmux send-key` only for named keys (arrows/enter/ctrl+c); letters via
  `cmux send -- "q"`. Test pane: workspace:1 surface:2.
- Uncompressed kitty frames stall on occluded panes → o=z is mandatory.

## Decision log

- D1: Reuse the pre-existing empty git repo at ~/code/github.com/0x63616c/pacmantui (no commits, clean).
- D2: Three parallel research agents own rendering / mechanics / tables; independent
  reviewer to verify table transcription before implementation relies on it.
- D3: Real-terminal validation will use a second cmux pane in workspace:1; screenshots
  must be pixel captures (cmux-cua or screencapture), not text buffers.
