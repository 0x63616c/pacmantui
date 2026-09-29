# pacmantui — progress & operating log

Goal: complete, polished Pac-Man in Rust with real pixel graphics inside Ghostty
via the Kitty Graphics Protocol. Faithful to The Pac-Man Dossier. Data-driven maps.
Repo: github.com/0x63616c/pacmantui (private).

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
- [ ] Research: rendering (agent, in flight) → docs/research/rendering.md
- [ ] Research: dossier mechanics (agent, in flight) → docs/research/dossier-mechanics.md
- [ ] Research: reference tables A.1/A.2 (agent, in flight) → docs/research/tables.md + tables.json
- [ ] Rendering prototype validated in real Ghostty/cmux pane
- [ ] Implementation plan + architecture + acceptance matrix (docs/plan/)
- [ ] Module contracts defined; parallel implementation delegated
- [ ] Core sim (movement, ghosts, modes, house, collisions, scoring)
- [ ] Maps: classic + custom; map format spec + validation
- [ ] Menus/flow, persistence, difficulty (Table A.2)
- [ ] Test suite incl. table traceability; CI green
- [ ] Real-terminal playtest + visual acceptance (screenshots in docs/validation/)
- [ ] Final push, README, handoff

## Decision log

- D1: Reuse the pre-existing empty git repo at ~/code/github.com/0x63616c/pacmantui (no commits, clean).
- D2: Three parallel research agents own rendering / mechanics / tables; independent
  reviewer to verify table transcription before implementation relies on it.
- D3: Real-terminal validation will use a second cmux pane in workspace:1; screenshots
  must be pixel captures (cmux-cua or screencapture), not text buffers.
