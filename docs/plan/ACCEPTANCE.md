# Acceptance matrix

Status: ☐ open · ◐ in progress · ☑ done (with evidence link). "Verify" states the
required KIND of proof; text captures never count for graphics items.

## Rendering
| # | Requirement | Verify | Status |
|---|---|---|---|
| R1 | Real pixel graphics via kitty protocol in Ghostty/cmux | pixel screenshot of live pane | ☑ smoke test docs/validation/00-kitty-smoke.md; game proof pending |
| R2 | Opaque #000000 across viewport incl. menus/HUD/letterbox | screenshots of every screen + pixel sampling | ☐ |
| R3 | No flicker/white flash/residual images | frame-sequence capture during motion | ☐ |
| R4 | Correct aspect, integer scaling, centering, letterboxing | screenshot + geometry check vs 224x288 reference | ☐ |
| R5 | Resize handled safely w/o losing game state | resize pane mid-game, screenshots before/after | ☑ window 1727x998→1200x700→back mid-pause, identical state: evidence/21-resize-small-paused.png, evidence/22-resize-restored-paused.png (docs/validation/03-live-gameplay.md) |
| R6 | Min/recommended pane size documented + small-pane behavior | README + screenshot at small size | ☐ |
| R7 | Clean exit/panic: cursor, raw mode, alt screen, images freed | scripted checks + terminal state after quit/Ctrl-C/panic | ◐ quit paths (Esc→menu→q, pause→q) and three Ctrl-C exits clean, `pgrep` empty, no corruption (docs/validation/03-live-gameplay.md §Checks without figures); panic-path check pending |
| R8 | Classic maze/sprites match arcade reference imagery | side-by-side vs documented reference images | ☐ |

## Gameplay fidelity (dossier traceability in TRACEABILITY.md)
| # | Requirement | Verify | Status |
|---|---|---|---|
| G1 | Movement, buffered turns, cornering, wall rules | unit+replay tests, live play | ☐ |
| G2 | Pellets/energizers/score/lives/extra life/fruit | tests + live play | ◐ tests (TRACEABILITY.md scoring rows) + live: fruit figs 10/11/16, energizers fig 18, score/lives/pellets figs 10–23 (docs/validation/03-live-gameplay.md); extra-life live demo pending (test-covered only) |
| G3 | Four ghosts w/ dossier targeting incl. Pinky/Inky up-bug | table-driven unit tests per ghost | ☐ |
| G4 | Scatter/chase schedule + reversals per level | tick-exact tests | ☐ |
| G5 | Frightened: random turns, speeds, flashing, 200-1600 chain | seeded tests | ☑ seeded tests (TRACEABILITY.md fright rows, green at fa29a31) + live: chain popup/blue ghosts/eyes fig 11, white flash phase fig 24 (docs/validation/03-live-gameplay.md) |
| G6 | House: dot counters, global counter, timeout release, eyes return, revival | scenario tests | ☐ |
| G7 | Tunnel slowdown, red-zone up-restriction, Cruise Elroy (incl. Clyde-in-house rule) | scenario tests | ☐ |
| G8 | Collision = shared tile; death sequence; pellets preserved after death | tests + live play | ☑ tests (TRACEABILITY.md collision/death rows) + live: death→respawn with eaten corridors preserved fig 20 (docs/validation/03-live-gameplay.md); death animation evidence/07-death-animation.png |
| G9 | Level clear animation, progression, table rows ≥21+, safe level-256 behavior | progression tests | ☑ sequence/progression tests (TRACEABILITY.md sequences rows) + live: clear sequence figs 13/17, level-2 READY with score carry and Table A.1 fruit strip figs 14/18 (docs/validation/03-live-gameplay.md) |
| G10 | Hard difficulty from Table A.2 (not ad-hoc multipliers) | table tests + live play | ☐ |
| G11 | Complete a level via real/legal-replay inputs, no state cheats | recorded evidence | ☑ both maps cleared via committed legal-input replays (bot-authored, re-verified headlessly at fa29a31: classic 4600/4558t, vertigo 6360/5743t): figs 12/13/17, docs/validation/replays/, honesty note in docs/validation/03-live-gameplay.md |

## Application
| # | Requirement | Verify | Status |
|---|---|---|---|
| A1 | Menu flow: map+difficulty select, controls, loading, ready, pause/resume, restart, return-to-menu, quit | live walkthrough screenshots | ◐ every path exercised live with real keys: map/difficulty cycling (CLASSIC↔VERTIGO, NORMAL↔HARD, restored), HIGH SCORES + CONTROLS screens opened/exited, menu quit to clean prompt (docs/validation/03-live-gameplay.md §Checks without figures); captured: menu fig 19 + evidence/03-menu-classic-normal.png, ready figs 14/18/20/23, pause/resume figs 15/21/22, restart fig 23, return-to-menu fig 19; scores/controls/loading sub-screens exercised but uncaptured (visible-tab constraint), so the screenshot KIND of proof is incomplete |
| A2 | Persistence: scores+settings, corrupt/missing tolerated, custom-map scores separated | unit tests + manual corrupt-file test | ☐ |
| A3 | Loading reflects real initialization | code review | ☐ |

## Maps
| # | Requirement | Verify | Status |
|---|---|---|---|
| M1 | Versioned documented map format | docs/map-format.md review | ☐ |
| M2 | Classic map faithful (geometry, 240+4, tunnels, house) | vs dossier maze transcription + reference image | ☐ |
| M3 | Genuinely different custom map, playable | spec + live play | ☑ spec maps/custom.pmtoml (Vertigo, per docs/map-format.md) + live play through a full level clear: figs 16/17/18, docs/validation/03-live-gameplay.md |
| M4 | Validation: reachability, tunnel pairing, house access, entity-specific rules; useful errors | malformed-map test corpus | ☐ |
| M5 | External map file loading via menu | live test | ☐ |

## Engineering
| # | Requirement | Verify | Status |
|---|---|---|---|
| E1 | fmt/clippy -D warnings/test/release-build green locally and in CI | CI run link | ☐ |
| E2 | Table A.1/A.2 fully transcribed w/ provenance, independently reviewed | review record | ◐ agents in flight |
| E3 | Traceability: rule→impl→independent test | TRACEABILITY.md complete | ☑ docs/plan/TRACEABILITY.md (complete for rules/sim/map at fa29a31, cross-linked to review-rules.md + review-sim.md) |
| E4 | Replay/scenario tooling, honest labeling | code review | ☐ |
| E5 | Regression test per significant bug | test list | ☐ |
| E6 | Repo pushed, CI green, README complete, evidence committed | remote check | ☐ |
