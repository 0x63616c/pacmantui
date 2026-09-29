# pacmantui architecture

Single binary crate, modules with strict dependency direction (top may use bottom,
never the reverse):

```
main ─▶ app (menus/state machine/loop) ─▶ render (kitty gfx, terminal lifecycle)
                 │                              ▲ reads RenderState snapshots only
                 ▼
            sim (deterministic game core) ─▶ rules (reference tables) 
                 │                        ─▶ map (parsing/validation)
                 ▼
            replay (recorded inputs)
```

## Module contracts

### `sim` — deterministic game simulation
- Pure: no terminal, no clock, no I/O, no float nondeterminism across platforms
  (prefer integer/fixed-point where the original is integer; f32 allowed only for
  euclidean-distance comparisons mirroring the arcade, which are deterministic).
- API sketch: `Game::new(map: &Map, rules: &Rules, seed: u64) -> Game`;
  `Game::tick(&mut self, input: InputFrame) -> Vec<Event>`; getters expose a
  `RenderState` snapshot (actor pixel positions, animation phases, board state,
  score/lives/level, mode timers).
- One `tick` = one arcade frame (60.606060 Hz nominal). All rule timings in ticks.
- Seeded PRNG for frightened-mode turns; identical (map, rules, seed, inputs) ⇒
  identical event stream (replay determinism invariant).

### `map` — parsing & validation
- Versioned text/RON/TOML map format (spec in docs/map-format.md, decided with data
  team). Loads classic + custom from embedded assets and external files.
- Validation: dimensions, tile legality, spawn presence/legality, door/house
  geometry, tunnel pairing, pellet reachability BY PAC-MAN under movement rules,
  ghost route existence house→board→house (respecting one-way/up-restrictions),
  scatter target validity. Errors are typed and human-readable.

### `rules` — reference tables & difficulty
- Table A.1 / A.2 and schedule tables as static data, transcribed from the Dossier
  (docs/research/tables.json is the provenance record; the Rust constants must be
  independently cross-checked against it by a reviewer, and tests compare against
  expectations hand-derived from the docs, not from the constants themselves).
- `Rules::classic(level, difficulty)` resolves per-level parameters (speeds in
  percent, frightened ticks/flashes, elroy thresholds, fruit, scatter/chase
  schedule, dot counters). Custom maps: pellet-dependent thresholds scale by
  documented rule (docs/map-format.md), classic values untouched for classic map.

### `app` — application state machine & flow
- States: Boot/Loading → Menu (map select, difficulty, controls, high scores) →
  Ready → Playing → Paused → LifeLost → LevelClear → GameOver → back to Menu.
- Owns fixed-timestep loop: accumulator over monotonic clock; sim ticks at fixed
  rate regardless of render rate; input sampled per tick; rendering never mutates
  sim.
- Persistence: high scores + settings in platform config dir, corrupt/missing
  files tolerated; classic and custom-map score tables separated by map id/hash.

### `render` — kitty graphics + terminal lifecycle
- Owns: raw mode, alternate screen, kitty protocol I/O, cell-size query, resize
  (SIGWINCH), letterboxing/integer scaling, opaque #000000 background everywhere,
  cleanup on exit AND panic (hook restores terminal, deletes images).
- Input: crossterm event stream → logical `InputFrame`s.
- Never blocks the sim; a slow terminal drops frames, not ticks.
- Internals per docs/research/rendering.md (framebuffer strategy TBD by prototype).

### `replay`
- Records (map id, rules id, seed, per-tick inputs); replays through `sim` for
  tests and for reaching deep states honestly. Used by integration tests.

## Deepening pass (post-validation)

Four seams named after live validation, each replacing an implicit contract
that had already produced (or nearly produced) a bug (glossary: CONTEXT.md):

- **FrameLayout** (`render/layout.rs`) — one owner of the map-grid→frame
  mapping (HUD padding rows, maze origin, HUD anchors); pad arithmetic had
  been re-derived per draw site and broke once (custom-map HUD overprint).
- **Compositor as test surface** (`render/compose.rs`) — the pure-CPU
  compositor is the render module's interface; prod tty and tests cross the
  same seam, retiring the parallel `test_api` surface that could drift.
- **Sequence PhasePolicy** (`sim/sequence.rs`) — what each frozen-play phase
  means (subsystems/visibility/timing) declared once and interpreted by tick
  and snapshot; both post-validation sim bugs were rows transcribed into one
  interpreter but not the other.
- **GameSession / InputSource** (`app/session.rs`) — the session loop
  (fixed-tick accumulator, recording, pause, end conditions, persistence
  policy) behind a tty-free interface; live-vs-replay is one seam with two
  adapters instead of four scattered conditionals, and overlay flags travel
  as frame data instead of sticky renderer state.

## Testing strategy (summary; details in TESTPLAN.md)
- Unit: per-module. Table tests assert against values hand-copied from
  docs/research/tables.md by a DIFFERENT agent than the one writing rules code.
- Integration: scripted replays for cornering, release counters, elroy, fruit,
  death/restart, level clear, difficulty differences.
- Property: no wall traversal, single-scoring collectibles, mode-transition
  legality, replay determinism.
- Render tests: golden-image tests of composed framebuffers (pure CPU, no tty);
  real-terminal acceptance is manual+scripted via cmux (docs/validation/).
