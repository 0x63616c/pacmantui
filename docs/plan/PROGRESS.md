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
- [ ] W1-SIM (agent relaunched 2026-09-28): full arcade rules engine → src/sim, tests/sim_*
- [x] W2-APP/REPLAY (lead): app state machine, menus glue, persistence, fixed-timestep
      loop, CLI (--map/--replay/--record), replay text format + tests (app awaits sim)
- [ ] W2-REVIEW: independent sim/mechanics review after sim lands
- [ ] Test suite incl. table traceability matrix (docs/plan/TRACEABILITY.md); CI green
- [ ] Real-terminal playtest + visual acceptance (screenshots in docs/validation/)
- [ ] Final push, README, handoff

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
