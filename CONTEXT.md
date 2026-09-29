# pacmantui — domain glossary

Terms with one owner in this codebase. Plan docs: docs/plan/ARCHITECTURE.md
(module contracts + deepening pass), docs/plan/PROGRESS.md (log),
docs/plan/ACCEPTANCE.md, docs/plan/TRACEABILITY.md.

- **FrameLayout** (`src/render/layout.rs`) — the map-grid→frame mapping: HUD
  padding rows, frame dimensions, maze band origin, HUD anchors. Every draw
  site and size check queries it; nobody does pad arithmetic locally.
- **Compositor** (`src/render/compose.rs`) — the pure-CPU module that turns a
  `Maze` + `RenderState` + `Overlay` into the finished pixel `Frame`. Its
  interface is the render layer's test surface; the kitty `Renderer` is the
  tty adapter on the same seam.
- **PhasePolicy** (`src/sim/sequence.rs`) — the declarative table saying, for
  each `Sequence` phase, which subsystems keep running while play is frozen,
  which actors are visible, and what ends the phase. `Game::tick` and
  `Game::render_state` interpret the table; no phase row is hand-listed twice.
- **GameSession / InputSource** (`src/app/session.rs`) — one play of one game
  as a tty-free module: input sourcing, fixed-tick accumulator, recording,
  pause, end conditions (`PlayEnd`), persistence policy. `InputSource` is the
  live-vs-replay seam (two adapters; replay sessions never persist);
  `play_once` in `src/app/mod.rs` is only terminal wiring around it.
