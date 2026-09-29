# Pac-Man Dossier — Reference Table Transcriptions

Transcribed from *The Pac-Man Dossier* by Jamey Pittman.

Sources used:

- **PDF**: `The Pac-Man Dossier.pdf` (tralvex.com mirror) — a browser printout of the
  original `home.comcast.net/~jpittman2/pacman/pacmandossier.html` page, print-dated
  2015-01-04. 47 PDF pages (printed footer numbering "N of 48").
- **HTML mirror (authoritative)**: <https://pacman.holenet.info/> — the complete
  version, "version 1.0.27, August 11, 2015". Byte-identical copy saved during this
  research session. Where the two differ, the HTML value is used (the PDF is an older,
  physically clipped printout).

All four gameplay data tables in the document are transcribed below: Table A.1 and
Table A.2 from Appendix A, plus the two data tables that appear in the body (the
scatter/chase schedule and the speed summary). No other data-bearing tables exist in
the document — every other `<table>` element in the HTML is page layout, chapter-heading
decoration, the table of contents, the glossary (definition list), or the FAQ.

---

## Table A.1 — Level Specifications

- **Source**: Appendix A "Reference Tables", HTML anchor `#LvlSpecs`; PDF page 38 of 47
  (printed footer "40 of 48").
- **Full source title (HTML)**: "Table A.1 — Level Specifications (100% speed =
  75.75757625 pixels/sec)". The parenthetical speed definition appears **only** in the
  HTML mirror; the older PDF printout titles it just "Table A.1 — Level Specifications".
- **Units**:
  - All `Speed` columns: percentage of full speed, where 100% = 75.75757625 pixels/sec
    (definition from the HTML table title; the body section "Speed", anchor `#CH2_Speed`,
    describes the same percentages as fractions of Pac-Man's maximum speed). Note that
    Elroy 2 speed is 105% on levels 5+, i.e. speeds may exceed 100%.
  - `~` prefix: the source marks every value in the **Pac-Man Dots Speed** and **Fright
    Pac-Man Dots Speed** columns with `~` (approximate). These are effective average
    speeds while eating dots — per the body "Speed" section, Pac-Man stops for one frame
    (1/60 s) each regular dot eaten (three frames for an energizer), which lowers his
    average speed by roughly ten percent.
  - `Bonus Points`: points awarded for eating the bonus symbol.
  - `Elroy 1/2 Dots Left`: count of dots remaining in the maze at which Blinky enters
    "Cruise Elroy" stage 1 / stage 2 (dots).
  - `Fright. Time`: seconds of frightened (blue) time after an energizer.
  - `# of Flashes`: number of white flashes ghosts make before frightened mode ends.
  - `–` (en dash): cell intentionally blank in the source. On levels 17, 19, 20, and 21+
    the ghosts never turn blue at all (per Ch. 2 "Frightening Behavior": frightened time
    shrinks with level until "they no longer turn blue at all (they still reverse
    direction, however)"). All five fright-related columns are dashed on those rows.
- **Level-range interpretation**: rows 1–20 are individual levels; the "21+" row applies
  to level 21 and every level thereafter (the game has no ending; parameters stop
  changing at 21+). Note the deliberate exceptional value: Pac-Man's speed **drops back
  to 90%** (dots ~79%) at 21+, while ghosts stay at 95%.
- **Footnotes/asterisks**: the source table has none, other than the title parenthetical
  reproduced above and the `~` markers noted above.

| Level | Bonus Symbol | Bonus Points | Pac-Man Speed | Pac-Man Dots Speed | Ghost Speed | Ghost Tunnel Speed | Elroy 1 Dots Left | Elroy 1 Speed | Elroy 2 Dots Left | Elroy 2 Speed | Fright. Pac-Man Speed | Fright Pac-Man Dots Speed | Fright Ghost Speed | Fright. Time (in sec.) | # of Flashes |
|------:|:-------------|-------------:|--------------:|-------------------:|------------:|-------------------:|------------------:|--------------:|------------------:|--------------:|----------------------:|--------------------------:|-------------------:|-----------------------:|-------------:|
| 1     | Cherries     | 100          | 80%           | ~71%               | 75%         | 40%                | 20                | 80%           | 10                | 85%           | 90%                   | ~79%                      | 50%                | 6                      | 5            |
| 2     | Strawberry   | 300          | 90%           | ~79%               | 85%         | 45%                | 30                | 90%           | 15                | 95%           | 95%                   | ~83%                      | 55%                | 5                      | 5            |
| 3     | Peach        | 500          | 90%           | ~79%               | 85%         | 45%                | 40                | 90%           | 20                | 95%           | 95%                   | ~83%                      | 55%                | 4                      | 5            |
| 4     | Peach        | 500          | 90%           | ~79%               | 85%         | 45%                | 40                | 90%           | 20                | 95%           | 95%                   | ~83%                      | 55%                | 3                      | 5            |
| 5     | Apple        | 700          | 100%          | ~87%               | 95%         | 50%                | 40                | 100%          | 20                | 105%          | 100%                  | ~87%                      | 60%                | 2                      | 5            |
| 6     | Apple        | 700          | 100%          | ~87%               | 95%         | 50%                | 50                | 100%          | 25                | 105%          | 100%                  | ~87%                      | 60%                | 5                      | 5            |
| 7     | Grapes       | 1000         | 100%          | ~87%               | 95%         | 50%                | 50                | 100%          | 25                | 105%          | 100%                  | ~87%                      | 60%                | 2                      | 5            |
| 8     | Grapes       | 1000         | 100%          | ~87%               | 95%         | 50%                | 50                | 100%          | 25                | 105%          | 100%                  | ~87%                      | 60%                | 2                      | 5            |
| 9     | Galaxian     | 2000         | 100%          | ~87%               | 95%         | 50%                | 60                | 100%          | 30                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 10    | Galaxian     | 2000         | 100%          | ~87%               | 95%         | 50%                | 60                | 100%          | 30                | 105%          | 100%                  | ~87%                      | 60%                | 5                      | 5            |
| 11    | Bell         | 3000         | 100%          | ~87%               | 95%         | 50%                | 60                | 100%          | 30                | 105%          | 100%                  | ~87%                      | 60%                | 2                      | 5            |
| 12    | Bell         | 3000         | 100%          | ~87%               | 95%         | 50%                | 80                | 100%          | 40                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 13    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 80                | 100%          | 40                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 14    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 80                | 100%          | 40                | 105%          | 100%                  | ~87%                      | 60%                | 3                      | 5            |
| 15    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 100               | 100%          | 50                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 16    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 100               | 100%          | 50                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 17    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 100               | 100%          | 50                | 105%          | –                     | –                         | –                  | –                      | –            |
| 18    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 100               | 100%          | 50                | 105%          | 100%                  | ~87%                      | 60%                | 1                      | 3            |
| 19    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 120               | 100%          | 60                | 105%          | –                     | –                         | –                  | –                      | –            |
| 20    | Key          | 5000         | 100%          | ~87%               | 95%         | 50%                | 120               | 100%          | 60                | 105%          | –                     | –                         | –                  | –                      | –            |
| 21+   | Key          | 5000         | 90%           | ~79%               | 95%         | 50%                | 120               | 100%          | 60                | 105%          | –                     | –                         | –                  | –                      | –            |

Fright-time sanity notes (values double-checked against both sources): the fright-time
sequence is non-monotonic — levels 6, 10, and 14 jump back up (5 s, 5 s, 3 s) and level
14 has 5 flashes; 1-second levels (9, 12, 13, 15, 16, 18) all have 3 flashes; every
level with ≥2 seconds has 5 flashes; levels 17, 19, 20, 21+ have no frightened period
at all (dash, not zero, in the source).

---

## Table A.2 — Difficulty Specifications

- **Source**: Appendix A "Reference Tables", HTML anchor `#DiffSpecs`; PDF page 39 of 47
  (printed footer "41 of 48").
- **What it is**: the mapping between normal-difficulty levels and hard-difficulty
  levels (a factory PCB option), with the bonus symbol shown on each. The table is
  preceded by a prose paragraph, summarized here: there is a spot on the Pac-Man PCB
  where two solder pads can be joined to set "hard" difficulty. The only gameplay
  difference is that five levels — 1, 3, 6, 19, and 20 — are eliminated from play. The
  bonus-symbol sequence is *not* eliminated, so symbols lag behind the true level (e.g.
  hard's first board is really level 2 gameplay but displays cherries, and bonus point
  values follow the displayed symbol, not the level). A machine's difficulty setting can
  be identified from the attract-mode demo: in normal difficulty Inky captures Pac-Man
  in the lower-left of the maze; with the hard jumper connected, Clyde captures him near
  the same spot.
- **Units/interpretation**: `Normal` / `Hard` columns are level numbers (the "gameplay
  level", i.e. the Table A.1 row in effect). `Normal Bonus` / `Hard Bonus` are the bonus
  symbol displayed, with the source's ordinal ("Peach 1", "Key 4") meaning the Nth
  appearance of that symbol in the sequence. `–` = that level does not exist in hard
  difficulty (it is skipped).
- **Level-range interpretation**: rows 1–20 individual, "21+" onward.
- **Footnotes/asterisks**: none in the table itself (the prose paragraph above is the
  table's only annotation).

| Normal | Normal Bonus | Hard | Hard Bonus |
|-------:|:-------------|-----:|:-----------|
| 1      | Cherries     | –    | –          |
| 2      | Strawberry   | 2    | Cherries   |
| 3      | Peach 1      | –    | –          |
| 4      | Peach 2      | 4    | Strawberry |
| 5      | Apple 1      | 5    | Peach 1    |
| 6      | Apple 2      | –    | –          |
| 7      | Grapes 1     | 7    | Peach 2    |
| 8      | Grapes 2     | 8    | Apple 1    |
| 9      | Galaxian 1   | 9    | Apple 2    |
| 10     | Galaxian 2   | 10   | Grapes 1   |
| 11     | Bell 1       | 11   | Grapes 2   |
| 12     | Bell 2       | 12   | Galaxian 1 |
| 13     | Key 1        | 13   | Galaxian 2 |
| 14     | Key 2        | 14   | Bell 1     |
| 15     | Key 3        | 15   | Bell 2     |
| 16     | Key 4        | 16   | Key 1      |
| 17     | Key 5        | 17   | Key 2      |
| 18     | Key 6        | 18   | Key 3      |
| 19     | Key 7        | –    | –          |
| 20     | Key 8        | –    | –          |
| 21+    | Key 9        | 21+  | Key 4      |

Other appendices (no further tables to transcribe): Appendix B "Easter Eggs & Tricks"
(prose only), Appendix C "Hardware Information" (cabinet/PCB specs and operator DIP
reference — no gameplay timing), Appendix D "Vintage Pac-Man Guides" (book scans).

---

## Scatter/Chase Schedule (body table, Chapter 2 "Scatter, Chase, Repeat...")

- **Source**: Chapter 2, section "Scatter, Chase, Repeat...", HTML anchor
  `#CH2_Scatter_Chase_Repeat`; PDF page 12 of 47 (printed footer "14 of 48").
- **Units**: the source states "all values are in seconds". `1/60` is one frame (the
  scatter period exists but lasts a single frame, appearing as a simple reversal).
  `indefinite` = ghosts remain in chase mode for the rest of the level.
- **Level-range interpretation**: three level bands — Level 1, Levels 2–4, Levels 5+
  (5 and beyond). Rows are the eight consecutive mode phases from level start, in order.
- **Context from the accompanying prose** (same section): the level starts in the first
  scatter period; the scatter/chase timer is **reset** whenever a life is lost or a
  level is completed; the timer is **paused** while ghosts are frightened and resumes
  when frightened mode ends (ghosts return to the mode they were in).
- **Footnotes/asterisks**: none.

| Mode    | Level 1    | Levels 2–4 | Levels 5+  |
|:--------|-----------:|-----------:|-----------:|
| Scatter | 7          | 7          | 5          |
| Chase   | 20         | 20         | 20         |
| Scatter | 7          | 7          | 5          |
| Chase   | 20         | 20         | 20         |
| Scatter | 5          | 5          | 5          |
| Chase   | 20         | 1033       | 1037       |
| Scatter | 5          | 1/60       | 1/60       |
| Chase   | indefinite | indefinite | indefinite |

---

## Speed Summary (body table, Chapter 2 "Speed")

- **Source**: Chapter 2, section "Speed", HTML anchor `#CH2_Speed`; PDF page 14 of 47
  (printed footer "16 of 48"). The section says this information is also contained in
  Table A.1 — it is a per-level-band condensation of A.1's speed columns and agrees with
  A.1 cell-for-cell.
- **Units**: percentage of maximum (full) speed, as in Table A.1. `NORM DOTS` /
  `FRIGHT DOTS` are the `~` approximate average-while-eating-dots values. `–` = no
  frightened mode on those levels.
- **Level-range interpretation**: bands 1, 2–4, 5–20, 21+.
- **Prose facts attached to this table** (same section, useful for implementation):
  Pac-Man stops moving for one frame (1/60 s) per regular dot eaten and for three
  frames per energizer; ghosts' normal speed is slightly below Pac-Man's until level 21;
  tunnel travel cuts ghost speed roughly in half; at level 21+ Pac-Man slows back to 90%
  for the remainder of the game.
- **Footnotes/asterisks**: none.

| LEVEL | Pac-Man NORM | Pac-Man NORM DOTS | Pac-Man FRIGHT | Pac-Man FRIGHT DOTS | Ghost NORM | Ghost FRIGHT | Ghost TUNNEL |
|:------|-------------:|------------------:|---------------:|--------------------:|-----------:|-------------:|-------------:|
| 1     | 80%          | ~71%              | 90%            | ~79%                | 75%        | 50%          | 40%          |
| 2 – 4 | 90%          | ~79%              | 95%            | ~83%                | 85%        | 55%          | 45%          |
| 5 – 20| 100%         | ~87%              | 100%           | ~87%                | 95%        | 60%          | 50%          |
| 21+   | 90%          | ~79%              | –              | –                   | 95%        | –            | 50%          |

(The source renders this with a two-row header: "PAC-MAN SPEED" spanning NORM / NORM
DOTS / FRIGHT / FRIGHT DOTS and "GHOST SPEED" spanning NORM / FRIGHT / TUNNEL.)

---

## Data that exists only as prose (not transcribed here)

For completeness of the survey — these gameplay values appear in the dossier **only in
prose**, not in any table, so per this document's scope they are left to the mechanics
research notes:

- Ghost-house dot counters / exit rules: Ch. 2 "Home Sweet Home" (anchor
  `#CH2_Home_Sweet_Home`).
- Bonus fruit appearance triggers (dots eaten) and fruit display duration: Ch. 1
  prose (referenced near the bonus-symbol discussion).
- Ghost-eating point values (200/400/800/1600) and other scoring: Ch. 1 prose.

---

## Verification notes

- **PDF pages viewed as page images** (Read tool on the PDF, not text extraction):
  PDF pages 11–14 (Modus Operandi through Speed/Cornering — includes the scatter/chase
  table on p. 12 and the speed table on p. 14) and PDF pages 38–39 (Appendix A: Table
  A.1 on p. 38, Table A.2 on p. 39). `pdftotext -layout` output was used only to locate
  page numbers, never as a source of cell values.
- **HTML mirror**: fetched live from `https://pacman.holenet.info/` during this session;
  it was byte-identical (MD5 `dab7725d…`) to the locally saved copy. Tables were parsed
  cell-by-cell with an HTML table parser (windows-1252 decoding), then every cell was
  compared against the PDF page images.
- **Discrepancies found (HTML value adopted in all cases)**:
  1. **A.1 title**: the HTML (v1.0.27, 2015-08-11) includes "(100% speed = 75.75757625
     pixels/sec)"; the PDF printout (2015-01-04) titles it plainly. Not a data conflict —
     the HTML is the newer, complete revision.
  2. **A.1 rightmost columns clipped in the PDF**: the printout is cut at the right page
     margin, physically truncating the "Fright. Time (in sec.)" and "# of Flashes"
     columns (only a sliver of the Fright.-Time header line is visible). Those two
     columns are transcribed from the HTML mirror only. All 14 PDF-visible columns of
     all 21 rows match the HTML cell-for-cell.
  3. **A.2 prose margin**: the same right-margin clipping truncates a few line-ends of
     the A.2 explanatory paragraph in the PDF; the full text was taken from the HTML.
     The A.2 table itself is fully visible in the PDF and matches the HTML exactly.
- **No genuine data conflicts** were found between PDF and HTML — every cell readable in
  both sources agrees.
- **Cells left null/dash**: none unreadable. The only empty cells are the source's own
  `–` dashes (A.1 fright columns on levels 17/19/20/21+; A.2 hard-difficulty rows for
  eliminated levels 1/3/6/19/20; speed-summary fright cells at 21+), which are
  deliberate "does not apply" markers, transcribed as `–` here and `null` in
  `tables.json`.
