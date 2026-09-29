# pacmantui

Pac-Man in your terminal — real pixel graphics inside [Ghostty](https://ghostty.org)
via the [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
No ASCII art, no Unicode blocks: the game composes a 224×288 framebuffer of
original 8-bit-style sprite art and ships it to the terminal as compressed
RGB images, integer-scaled and letterboxed on black.

Gameplay follows [The Pac-Man Dossier](https://pacman.holenet.info/) (Jamey
Pittman) as the source of truth for classic-mode mechanics — ghost targeting
quirks, scatter/chase schedules, house dot counters, Cruise Elroy, cornering,
the lot. See [Fidelity and deviations](#fidelity-and-deviations).

**Status: complete and validated** — live acceptance run in Ghostty with
pixel-screenshot evidence, arcade-fidelity comparison, and full
rule→impl→test traceability. See [Validation](#validation).

## Requirements

- A terminal implementing the kitty graphics protocol **and** pixel-size
  reporting: [Ghostty](https://ghostty.org) and [kitty](https://sw.kovidgoyal.net/kitty/)
  are known good (the live acceptance run was performed in Ghostty on
  macOS). tmux/screen are detected and refused (they break graphics
  passthrough).
- A window of at least 224×288 pixels for the classic map (the app prints the
  exact requirement, in cells for your font size, if the window is too small).
- Stable Rust (2024 edition) + Cargo. No other system dependencies.

## Build, run, test

```sh
cargo run --release            # play
cargo build --release --locked # build, then run ./target/release/pacmantui
cargo test --all-targets --all-features                        # full test suite
cargo fmt --check                                              # formatting
cargo clippy --all-targets --all-features -- -D warnings       # lints
```

The full CLI is `--map <FILE.pmtoml>` (repeatable), `--replay <FILE>`,
`--record <FILE>`, `--version`, `--help`.

## Controls

| Key | Action |
|---|---|
| Arrows / WASD | Move (turns are buffered, as in the arcade) |
| Enter | Select (menus), dismiss game over |
| P | Pause / resume |
| R | Restart game (while paused or at game over) |
| Esc | Pause, then return to menu; back (menus) |
| Q / Ctrl-C | Quit |

## Maps

Two maps ship embedded in the binary:

- **Classic** — the faithful 1980 maze, 244 pellets, arcade tunnel.
- **Vertigo** — an original 32×26 layout: low-center ghost house, twin
  vertical tunnels, a full-width central crossing.

Maps are data, not code: a versioned TOML + ASCII-grid format documented in
`docs/map-format.md`, validated on load (reachability, tunnel pairing, house
geometry, ghost return routes — with useful errors). Add your own:

```sh
pacmantui --map my-maze.pmtoml        # add to the menu for this run
# or drop it in ./maps/ or ~/.config/pacmantui/maps/ — valid files are
# picked up by the menu automatically
```

Pellet-dependent thresholds (Elroy, fruit triggers, house counters) scale to a
custom map's pellet count by the documented rule; classic values are untouched
on the classic map.

## Difficulty

- **Normal** — Table A.1 of the Dossier, level by level.
- **Hard** — the arcade's "hard" table (Table A.2 level elimination), not
  ad-hoc multipliers.

## Scores, settings, replays

High scores (top 10 per map × difficulty — custom-map tables never mix with
classic) and settings live in `~/.config/pacmantui/` (override with
`$PACMANTUI_CONFIG_DIR`; `$XDG_CONFIG_HOME` respected). Corrupt or missing
files are tolerated and rewritten.

Every game can be recorded and replayed deterministically — same seed, same
inputs, same game:

```sh
pacmantui --record game.replay   # write a replay of each finished game
pacmantui --replay game.replay   # watch it back through the real engine
```

The replay text format (v1) is documented in `src/replay/mod.rs`. Three
legal-input replays are committed as acceptance evidence — try

```sh
./target/release/pacmantui --replay docs/validation/replays/classic-level1-clear.replay
```

## Fidelity and deviations

Mechanics are implemented from the Dossier with a traceability matrix
(reference rule/table → implementation → independently-checked test):
`docs/plan/TRACEABILITY.md`. Research notes with citations live in
`docs/research/`. Deliberate deviations are few and documented, notably:

- **Level 256**: the arcade's kill screen is replaced by safe continuation on
  the final difficulty row.
- **Fruit duration**: picked in the ROM's 9–10 s range from the seeded PRNG.
- Timings the Dossier lacks (death/READY/level-clear frame counts, eyes
  speed, ghost-eaten pause semantics) use ROM-derived values with
  per-constant provenance comments in `src/sim/timings.rs`
  (`docs/research/arcade-supplements.md`).

## Validation

Real-terminal evidence lives in `docs/validation/`:

- `03-live-gameplay.md` — the live acceptance run: screenshots of fruit,
  fright chains, level clears, pause/resize/restart, both maps, plus the
  committed replays re-verified headlessly.
- `04-fidelity.md` — arcade-fidelity comparison: maze layout, pellets,
  start positions tile-exact and the palette exact vs reference imagery.
- `docs/plan/TRACEABILITY.md` — every rule and table value traced from its
  reference source to the implementation and an independently checked test.

## Limitations

- Sprite art is original, arcade-*inspired*, not pixel-identical (D10), and
  the tunnel side-pocket wall art / outer border differ cosmetically from
  the arcade (D15) — maze layout, pellets, and collision are tile-exact.
- Single-player HUD: no `2UP` area.
- Level 256 continues safely instead of reproducing the kill screen.
- Panic-path terminal cleanup is implemented (panic hook + drop) but was
  not exercised in the live acceptance run; quit and Ctrl-C paths were.
- The 10,000-point extra life is verified by tests only — attended play
  never reached it.
- Hard difficulty is covered by table tests plus one sim test; it was not
  played live.

## Architecture

Strict one-way module graph (`docs/plan/ARCHITECTURE.md`): `app` (menus,
fixed-timestep loop, persistence) → `render` (kitty protocol, terminal
lifecycle) and `sim` (pure, deterministic, integer-math rules engine) →
`rules` (transcribed reference tables) and `map`. The sim never touches the
terminal or the clock; render delays drop frames, never ticks.
