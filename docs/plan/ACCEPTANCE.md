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
| R5 | Resize handled safely w/o losing game state | resize pane mid-game, screenshots before/after | ☐ |
| R6 | Min/recommended pane size documented + small-pane behavior | README + screenshot at small size | ☐ |
| R7 | Clean exit/panic: cursor, raw mode, alt screen, images freed | scripted checks + terminal state after quit/Ctrl-C/panic | ☐ |
| R8 | Classic maze/sprites match arcade reference imagery | side-by-side vs documented reference images | ☐ |

## Gameplay fidelity (dossier traceability in TRACEABILITY.md)
| # | Requirement | Verify | Status |
|---|---|---|---|
| G1 | Movement, buffered turns, cornering, wall rules | unit+replay tests, live play | ☐ |
| G2 | Pellets/energizers/score/lives/extra life/fruit | tests + live play | ☐ |
| G3 | Four ghosts w/ dossier targeting incl. Pinky/Inky up-bug | table-driven unit tests per ghost | ☐ |
| G4 | Scatter/chase schedule + reversals per level | tick-exact tests | ☐ |
| G5 | Frightened: random turns, speeds, flashing, 200-1600 chain | seeded tests | ☐ |
| G6 | House: dot counters, global counter, timeout release, eyes return, revival | scenario tests | ☐ |
| G7 | Tunnel slowdown, red-zone up-restriction, Cruise Elroy (incl. Clyde-in-house rule) | scenario tests | ☐ |
| G8 | Collision = shared tile; death sequence; pellets preserved after death | tests + live play | ☐ |
| G9 | Level clear animation, progression, table rows ≥21+, safe level-256 behavior | progression tests | ☐ |
| G10 | Hard difficulty from Table A.2 (not ad-hoc multipliers) | table tests + live play | ☐ |
| G11 | Complete a level via real/legal-replay inputs, no state cheats | recorded evidence | ☐ |

## Application
| # | Requirement | Verify | Status |
|---|---|---|---|
| A1 | Menu flow: map+difficulty select, controls, loading, ready, pause/resume, restart, return-to-menu, quit | live walkthrough screenshots | ☐ |
| A2 | Persistence: scores+settings, corrupt/missing tolerated, custom-map scores separated | unit tests + manual corrupt-file test | ☐ |
| A3 | Loading reflects real initialization | code review | ☐ |

## Maps
| # | Requirement | Verify | Status |
|---|---|---|---|
| M1 | Versioned documented map format | docs/map-format.md review | ☐ |
| M2 | Classic map faithful (geometry, 240+4, tunnels, house) | vs dossier maze transcription + reference image | ☐ |
| M3 | Genuinely different custom map, playable | spec + live play | ☐ |
| M4 | Validation: reachability, tunnel pairing, house access, entity-specific rules; useful errors | malformed-map test corpus | ☐ |
| M5 | External map file loading via menu | live test | ☐ |

## Engineering
| # | Requirement | Verify | Status |
|---|---|---|---|
| E1 | fmt/clippy -D warnings/test/release-build green locally and in CI | CI run link | ☐ |
| E2 | Table A.1/A.2 fully transcribed w/ provenance, independently reviewed | review record | ◐ agents in flight |
| E3 | Traceability: rule→impl→independent test | TRACEABILITY.md complete | ☐ |
| E4 | Replay/scenario tooling, honest labeling | code review | ☐ |
| E5 | Regression test per significant bug | test list | ☐ |
| E6 | Repo pushed, CI green, README complete, evidence committed | remote check | ☐ |
