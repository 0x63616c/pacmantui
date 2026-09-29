# Rendering research: Kitty graphics protocol, Incredible Flappy's K mode, Ghostty

Research for pacmantui's pixel renderer (Pac-Man, 224x288 native, rendered in Ghostty).
Researched 2026-09-28. Sources:

- Kitty graphics protocol spec: <https://sw.kovidgoyal.net/kitty/graphics-protocol/> (read in full; section names cited below)
- Article "TUI Games in 80x24" (Ron Ilan, 2026-09-27): <https://www.incredible.rs/#blog/tui-games-in-80x24.md> — the SPA shell hides the content; the markdown source lives at `blog/tui-games-in-80x24.md` in <https://github.com/ronilan/incredible>
- Incredible Flappy source: <https://github.com/ronilan/incredible-flappy> @ `724f7214` (2026-09-27), cloned locally. The framework that actually emits the escape codes (`incredible` / `incredible_output_terminal`) lives in the **private** repo `ronilan/incredible-alpha` (pinned in Cargo.lock at `0d6a2720`), so its wire behavior was verified **empirically**: by `strings`-analysis of the released `incredible_flappy-terminal-macos-arm` v0.1.0 binary and by running that binary under a fake-kitty PTY (env `TERM=xterm-kitty`, `TERM_PROGRAM=ghostty`) and capturing 14 MB of raw output. Everything stated below about its escape sequences is from that capture, not guessed.
- Ghostty: source at tags `v1.3.1`/`main` (`src/terminal/kitty/*.zig`), release notes, GitHub issues (cited inline).

---

# 1. Kitty Graphics Protocol essentials

## 1.1 Escape sequence format

Every graphics command is an APC sequence (spec: "The graphics escape code"):

```
<ESC>_G<control data>;<payload><ESC>\
```

i.e. bytes `\x1b_G` + comma-separated `key=value` pairs + `;` + payload + `\x1b\\` (ST).
The payload is always **base64** (of pixel data, PNG data, a file path, or an SHM name,
depending on the control keys). Most terminals ignore APC, so it degrades safely.

**Chunking** (spec: "Remote client"): base64 data must be split into chunks of **at most
4096 bytes**, and every chunk except the last must have a length that is a **multiple of
4**. The first chunk carries the full control data plus `m=1`; middle chunks carry *only*
`m=1` (plus optionally `q`); the final chunk carries `m=0`:

```
\x1b_Ga=t,f=32,s=224,v=288,i=1,q=2,m=1;<4096 b64 chars>\x1b\
\x1b_Gm=1;<4096 b64 chars>\x1b\
...
\x1b_Gm=0;<final chunk>\x1b\
```

No other graphics escape may be interleaved mid-transmission. The cursor position that
matters for display is the position when the **final** chunk arrives. Terminals must not
display anything until the whole sequence is received and validated.

## 1.2 Transmission mediums (`t=`)

Spec: "The transmission medium". Default `t=d`.

| key | medium | notes for macOS + Ghostty |
|-----|--------|---------------------------|
| `t=d` | direct — data inside the escape code | Always works; the safe default. Recommended. |
| `t=f` | regular file; payload = base64 of the path | Works in Ghostty (POSIX). Terminal reads, does not delete. |
| `t=t` | temp file; terminal deletes it after reading | Path must live in a known temp dir and contain the string `tty-graphics-protocol`. Works in Ghostty. |
| `t=s` | POSIX shared memory object; payload = base64 of SHM name (`/name`, ≤ OS max) | Ghostty implements it via `shm_open`+read, then `shm_unlink` (v1.3.1 `src/terminal/kitty/graphics_image.zig` `readSharedMemory`, lines ~108-138). Unsupported on Windows. Caveat on macOS: `shm_open` names are limited to `PSHMNAMLEN` (31 chars) — keep names short. |

`t=f`/`t=t` support partial reads with `S=<size>`,`O=<offset>`. For a local game, `t=d`
is plenty fast (see §4 bandwidth math); `t=s` is the optimization escape hatch, not the
starting point.

## 1.3 Pixel formats (`f=`)

Spec: "Transferring pixel data".

- `f=32` — 32-bit RGBA, sRGB, 4 bytes/pixel. **This is the default.** Requires `s=`(width),`v=`(height).
- `f=24` — 24-bit RGB, 3 bytes/pixel. Requires `s=`,`v=`. 25% less data than RGBA.
- `f=100` — PNG. Width/height read from the PNG itself.
- `o=z` — optional zlib (RFC 1950) deflate of the payload, applicable to any `f=`. Compression happens *before* base64. With PNG + compression you must also send `S=<png size>`.

Tradeoffs for a game framebuffer sent every frame:

- `f=24` is the cheapest raw format for a fully opaque frame — pacmantui's frame is opaque, alpha buys nothing.
- `f=100` (PNG) costs a per-frame encode on the CPU and a decode in the terminal; only wins over slow links (SSH). Not worth it locally.
- `o=z` on `f=24` is very effective for flat-color pixel art (Pac-Man frames deflate ~10-30x) but adds per-frame compression latency; keep it as an option behind a flag, measure first.

## 1.4 Actions, image ids, placements

`a=` selects the action (spec: "Control data reference"):

- `a=t` — transmit data only (store under image id `i=`; nothing displayed).
- `a=T` — transmit **and** display at the cursor in one command.
- `a=p` — put/display a previously transmitted image (`a=p,i=10`).
- `a=q` — query: terminal tries to load but stores nothing; used for capability detection.
- `a=d` — delete (see below).
- `a=f`/`a=a`/`a=c` — animation frame transmit / animation control / frame compose (§1.8).

**Image ids**: `i=` is an arbitrary client-chosen 32-bit integer 1..4294967295 in a
*global* (per-terminal-screen) namespace. Re-transmitting data for an existing id
**deletes the old image and all its placements** first (spec note under "Display images
on screen") — the new data is *not displayed* until a new placement is made. If you can't
own the id space (screen shared with other programs), use `I=<number>` instead and the
terminal replies with the real id — irrelevant for a full-screen game that owns the tty.

**Placements**: each display of an image is a placement, optionally identified by
`p=<id>`; (image id, placement id) is unique. Sending a second placement with the same
`i` and `p` **replaces the first — the spec explicitly calls this out as the way to move
or resize placements "without flicker"**. Multiple `a=p` with `p=0` create *multiple*
placements of the same image.

**Deletes** (`a=d`, key `d`, spec: "Deleting images"): lowercase = remove placements but
keep the stored data (image can be re-placed without re-transmitting); **uppercase = also
free the image data** (if not still referenced e.g. from scrollback). Values relevant to us:

| `d=` | deletes |
|------|---------|
| `a`/`A` | all placements visible on screen (default if `d` omitted) |
| `i`/`I` | image with id `i=` (plus optional `p=` to hit one placement) |
| `n`/`N` | newest image with number `I=` |
| `c`/`C` | placements intersecting the cursor |
| `p`/`P` | placements intersecting cell `x=,y=` (1-based) |
| `z`/`Z` | placements with z-index `z=` |
| `r`/`R` | images with id in range [`x=`,`y=`] (kitty ≥ 0.33, Ghostty ≥ 1.2.0) |
| `f`/`F` | animation frames |

A delete arriving mid-chunked-upload aborts the upload. When quota pressure hits,
terminals preferentially evict images that have no placements.

## 1.5 Positioning and layout

Spec: "Controlling displayed image layout".

- Images render at the **current cursor position**, anchored to the top-left corner of the current cell. There is no absolute-pixel placement: you position with normal cursor addressing (`\x1b[<row>;<col>H`) and then place.
- `X=`,`Y=` — pixel offset *within the first cell* (must be smaller than the cell).
- `x=,y=,w=,h=` — source rectangle in pixels (crop of the transmitted image).
- `c=`,`r=` — display over that many columns/rows; the terminal **scales** the image to fit. One of them alone preserves aspect ratio; both together letterbox/pillarbox. Beware: the scaling filter is up to the terminal (typically GPU linear sampling) — for crisp pixel art, do integer nearest-neighbour scaling yourself and avoid `c=`/`r=`, or make the c×r box exactly match the pixel size.
- `z=` — 32-bit z-index. **Negative z draws under text**; z < INT32_MIN/2 (−1,073,741,824) draws even under non-default background colors. Overlapping same-z images: lower id is beneath.
- After placement the cursor moves right/down by the placement's cols/rows (undefined if that leaves the screen). **`C=1` suppresses all cursor movement** — use it for a game loop so placement never disturbs your cursor bookkeeping.
- **Flicker-free replacement**: the spec's own mechanisms are (a) re-place with same `i=`+`p=` to move/resize; (b) for new *content*, either transmit under a fresh id then place it and delete the old placement (double-buffer), or use `a=f` frame animation (§1.8). Retransmitting under the *same* id implicitly deletes its placements first, leaving a possible one-frame hole — wrap in synchronized output (mode 2026) and/or double-buffer to be safe. See §4.

**Relative placements** (spec: "Relative placements", kitty ≥ 0.31): `P=`,`Q=` make a
placement track a parent placement, offset by `H=`,`V=` cells. Ghostty stable 1.3.x does
not have this (landed on tip 2026-08, ships in 1.4.0) — don't rely on it.

**Unicode placeholders** (spec: "Unicode placeholders", kitty ≥ 0.28): transmit with
`q=2`, then create a *virtual* placement `a=p,U=1,i=<id>,c=<cols>,r=<rows>`; nothing is
drawn until you print the placeholder character **U+10EEEE** into cells. Each placeholder
cell encodes: image id in the **foreground color** (256-color = 8-bit ids, truecolor =
24-bit ids; 4th byte via a third diacritic), placement id in the **underline color**, and
its (row, col) inside the image via combining diacritics from `rowcolumn-diacritics.txt`
(U+0305 = 0, U+030D = 1, U+030E = 2, U+0310 = 3, …). Row/col diacritics can be omitted
and inherited from the cell to the left. The image then behaves *as text*: scrolls,
moves, and clips with the cells, and is moved/deleted by rewriting cells, not by graphics
commands. Virtual placements are only deletable via `d=i/I/r/R/n/N`.

## 1.6 Querying support and window metrics

- **Protocol detection** (spec: "Querying support and available transmission mediums"): send a 1×1 dummy query followed by DA1: `\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\` then `\x1b[c`. A `\x1b_Gi=31;OK\x1b\` (or error) response before the DA1 reply means graphics are supported; DA1-only means not. The same trick with `t=s`/`t=f` probes each medium. Terminals must answer `a=q` immediately, in FIFO order.
- **Cell/window pixel size** (spec: "Getting the window size"): primary method is `ioctl(0, TIOCGWINSZ)` — `ws_xpixel`/`ws_ypixel` give the text area size in pixels, `ws_row`/`ws_col` the grid; cell size = pixels ÷ cells. (Some terminals report 0 pixels; kitty, xterm, and Ghostty report real values.) Escape fallbacks: `\x1b[14t` → reply `\x1b[4;<height>;<width>t` (window pixels); `\x1b[16t` → reply `\x1b[6;<cellheight>;<cellwidth>t` (one cell, less widely supported but Ghostty handles both — v1.3.1 `src/terminal/stream.zig` lines 1755-1766).

## 1.7 Quiet mode

`q=1` suppresses OK responses, `q=2` suppresses failure responses too (spec:
"Suppressing responses"). For a game emitting graphics every frame, use `q=2` on
everything except the initial capability probe — otherwise the terminal floods your stdin
with `\x1b_Gi=..;OK\x1b\` after every transmit/placement.

## 1.8 Animation (`a=f`, `a=a`, `a=c`)

Spec: "Animation". An image becomes an animation by transmitting extra frames with
`a=f,i=<id>` (same format/chunking as `a=t`; frames may cover sub-rectangles
`x=,y=,s=,v=`, compose onto a previous frame `c=<frame>` or a flat RGBA background
`Y=<rgba>`, with per-frame gap `z=<ms>`). `a=a` controls playback: `s=1` stop, `s=2` run
awaiting frames, `s=3` loop; `c=<n>` makes frame n current (client-driven animation);
`v=` sets loop count. `a=c` composes rectangles between stored frames server-side.
**Not usable for pacmantui yet: unimplemented in every stable Ghostty as of 2026-09 (§3).**

## 1.9 Lifecycle: exit, alternate screen, scroll, resize

Spec: "Interaction with other terminal actions" + "Image persistence and storage quotas".

- **Reset** clears all visible images. **`\x1b[2J` (ED2) deletes all images** on screen, like text. Other text-erase commands do *not* touch graphics — use `a=d`.
- **Alternate screen** (mode 1049): the spec requires that on switching to the alt screen, all images in the alt screen are cleared, just as its text is — so alt-screen placements never survive an exit/re-entry cycle. Images on the main screen are untouched while you're in alt. So: run the game in the alt screen and most cleanup is automatic on exit; still send an explicit `\x1b_Ga=d,d=A\x1b\` (delete all placements **and** free data) before leaving, because leaving the alt screen removes placements but stored image *data* counts against quota until evicted.
- **Scrolling** moves images with the text; images crossing scroll-region margins are clipped. Irrelevant if you never scroll (fixed-viewport game in alt screen).
- **Resize** is not specified per se: placements stay anchored to their cells; your geometry math (cell size, centering) is invalidated. Handle SIGWINCH: recompute, delete, re-place (§4.4).
- **Quota**: terminals keep a storage quota (kitty: 320 MB/buffer; Ghostty: 320 MB default, `image-storage-limit` config, 0 disables the protocol). Oldest images are evicted when exceeded — one more reason to delete or overwrite ids instead of minting fresh ids forever at 60 fps (id churn is fine, unbounded *stored images* are not; overwriting the same ids avoids the issue entirely).

---

# 2. How Incredible Flappy's K mode works

Context from the article: Incredible Flappy is built on the `incredible` TUI framework;
pressing **K** in kitty/Ghostty swaps ANSI-art game elements for real PNG sprites
("Flappy into Fluffy"). The author notes he hit protocol limits "in two places" but
doesn't say where. The framework repo (`ronilan/incredible-alpha`) is private, so the
app-level code below is from the game repo @ `724f7214`, and the wire-level facts are
from the released v0.1.0 macOS binary (strings + PTY capture, see header).

## 2.1 Sprite storage and decoding (game side)

- Sprites are **PNG files compiled into the binary** with `include_bytes!` and decoded at element construction into raw RGBA via the `image` crate: `src/ui/assets.rs:4-14` (`decode_png` → `ImageData { bytes: Vec<u8> /* RGBA */, width_px, height_px }`); duplicated privately in `src/ui/elements/flying_cat.rs:102-112`.
- An `Image<S>` framework element holds one decoded sprite plus a size **in cells** (`.width(cells).height(cells)`), e.g. the bird: `src/ui/elements/flying_cat.rs:37-56` builds three `Image` children (`flying_cat_up`, `flying_cat_down`, `flying_cat_ready`), each 9×3 cells.
- Sprite *animation* is done by toggling visibility between pre-built Image elements, not by touching image data: `set_rising()` flips `showed()` between up/down wing sprites (`flying_cat.rs:73-85`). Same pattern everywhere: score digits are ten pre-decoded PNG digit images composed per value (`src/ui/elements/u16_image.rs:12-23, 65-89`), kitty-mode buttons/ghost-logos likewise (`src/ui/elements/image_button.rs`, `alien.rs`).
- K-mode is a persisted bool (`src/settings.rs:15,44,62`) mirrored into UI state; the app shows either text twins or image twins of each widget per phase (`src/ui/app.rs:348-490`), and `game.set_kitty()` restyles the panel (`src/ui/elements/flappy_engine.rs:371-377`).
- **Support detection is environment-based, no `a=q` handshake**: binary strings show the check reads `TERM`, `KITTY_WINDOW_ID`, `GHOSTTY_SOCKET`, `TERM_PROGRAM`/`TERM_PROGRAM_VERSION` against `kitty`/`ghostty`; `src/runtime.rs:15-19` then forces `state.kitty = false` when `Platform::output_provider().images()` is false. It also never queries pixel metrics (no `CSI 14t`/`16t`, no TIOCGWINSZ dependency for graphics) — it doesn't need them, because of how it places images (next section).

## 2.2 Wire protocol (framework, observed)

The framework's terminal backend (`incredible_output_terminal`, format strings at binary
offset `0x191e13-0x191e79`) uses the **Unicode placeholder** flavor of the protocol
exclusively. Captured sequences, exactly:

1. **Transmit** each sprite's RGBA once, direct medium, quiet, chunked:
   `\x1b_Ga=t,f=32,s=<w>,v=<h>,i=<id>,q=2,m=1;<b64>\x1b\` + `\x1b_Gm=1;<4096 b64>\x1b\`… + final `\x1b_Gm=0;…\x1b\`. Chunk payloads are exactly 4096 base64 chars (the spec max). No `t=` key (default direct), no compression, never `a=T`.
2. **Virtual placement** immediately after each transmit:
   `\x1b_Ga=p,U=1,i=<id>,c=<cols>,r=<rows>,q=2,S=1\x1b\` — the element's cell box; the terminal scales the sprite into it. (Quirk: `S=1` is spec'd as "size of data to read from a file" and is meaningless on `a=p`; kitty/Ghostty ignore it. Don't copy this.)
3. **Draw/move/animate = plain text.** Each visible image cell is printed as fg-color + placeholder + row/col diacritics, e.g. the first captured cell:
   `\x1b[38;2;0;0;1m\U0010EEEE̅̅` (fg truecolor R=0,G=0,B=1 → image id 1; U+0305,U+0305 → row 0, col 0; next cell `…̅̍` → row 0, col 1, etc.). The framework always uses `38;2` truecolor ids and always writes explicit row+col diacritics per cell. Movement between frames is just its normal cell-diff redraw relocating those characters. **No graphics escape is emitted per animation frame at all.**
4. **Delete**: `\x1b_Ga=d,d=i,i=<id>,q=2\x1b\` (lowercase i — drops the virtual placement, data eviction left to the terminal) when an image element is destroyed. None fired during normal gameplay in the capture.

Observed dynamics over a ~10 s session (splash → K toggle → game → pause): 85 transmits,
ids allocated sequentially 1..85, in four bursts corresponding to screen (re)builds — the
framework re-transmits every sprite of a screen each time the screen is constructed, even
identical PNGs (e.g. the same 113×58 sprite went up as ids 34, 58 and 82), ~13.8 MB of
the 14.1 MB capture was transmission. Per-frame cost during play is only text. Other
observations: no alt screen, no `\x1b[2J`, no synchronized output (mode 2026); cursor
hidden, mouse (1003/1006) and kitty keyboard protocol enabled; game logic ticks at 100 ms
(`flappy_engine.rs:53-63` `interval_ms: 100`).

## 2.3 Lessons for pacmantui

Worth copying: transmit sprites once and never per frame; `q=2` everywhere; sequential
app-owned ids; direct medium; RGBA from `image`-crate-decoded PNGs. Not worth copying:
the Unicode-placeholder placement model (it quantizes sprite position to **whole cells**
— fine for Flappy's chunky cell-based movement, wrong for Pac-Man where actors move in
1-pixel steps: an 8 px step per cell at 1× would make movement lurch cell-by-cell); the
per-screen retransmission (wasteful); the stray `S=1`; env-only detection (fine as a fast
path, but the `a=q` probe is the reliable answer).

---

# 3. Ghostty specifics

Ghostty is listed among terminals implementing the protocol on the spec page itself.
State as of **2026-09-28** (current stable = 1.3.1; 1.4.0 milestone still open with 29
issues, so everything "tip/1.4" below is not yet in a stable release):

| feature | stable 1.3.x | notes / source |
|---------|--------------|----------------|
| `a=t/T/p/q/d`, chunking, `q=` | yes | shipped since pre-1.0 (issue [#317](https://github.com/ghostty-org/ghostty/issues/317)) |
| mediums `t=d`, `t=f`, `t=t`, `t=s` | yes (POSIX; `t=s` unsupported on Windows) | `v1.3.1 src/terminal/kitty/graphics_image.zig` — `readSharedMemory` uses `shm_open`/`shm_unlink`; `t=f/t=t` enforce realpath, regular-file, temp-dir + `tty-graphics-protocol` naming rules |
| formats `f=24/32/100`, `o=z` | yes | same file; PNG via wuffs |
| z-index incl. negative-under-text | yes | z-index bug fixed in 1.2.0 ([release notes](https://ghostty.org/docs/install/release-notes/1-2-0), #7671) |
| `C=1`, `X/Y`, `x/y/w/h`, `c/r` scaling | yes | 1.2.0 also removed the old grid-size constraint (#7367) and fixed aspect-ratio bugs (#6673) |
| delete variants incl. `d=r/R` range | yes | range delete added 1.2.0 (#5957) |
| **Unicode placeholders (`U=1`)** | **yes** | implemented 2024-07 (issues [#2015](https://github.com/ghostty-org/ghostty/issues/2015), [#720](https://github.com/ghostty-org/ghostty/issues/720), closed 2024-07-31) — i.e. in every 1.x release. Known bug: placeholder images mis-scaled *inside tmux* on 1.3.1, fixed on tip ([#13056](https://github.com/ghostty-org/ghostty/issues/13056)) |
| **Animation `a=f`/`a=a`/`a=c`** | **NO — replies `ERROR: unimplemented action`** | verbatim in `v1.3.1 src/terminal/kitty/graphics_exec.zig:71-74`; confirmed by maintainer ([discussion #5218](https://github.com/ghostty-org/ghostty/discussions/5218), Jan 2025). Implemented on main 2026-08-21 ([#5255](https://github.com/ghostty-org/ghostty/issues/5255) closed, milestone 1.4.0; `graphics_animation.zig` on main) |
| Relative placements (`P/Q/H/V`) | no (tip/1.4.0) | [#13939](https://github.com/ghostty-org/ghostty/issues/13939) closed 2026-08-20 |
| `CSI 14t` / `CSI 16t`, TIOCGWINSZ pixels | yes | `v1.3.1 src/terminal/stream.zig:1755-1766`; Ghostty reports real `ws_xpixel/ws_ypixel` |
| Synchronized output (mode 2026) | yes | `v1.3.1 src/terminal/modes.zig:229` |
| Storage quota | 320 MB default | `graphics_storage.zig:58`; config key `image-storage-limit` (0 disables graphics entirely) |

Other Ghostty notes:

- Mitchell Hashimoto announced a "100% complete implementation of the Kitty Graphics Protocol — every feature, every option" on 2026-08-21 ([x.com/mitchellh/status/2090875695346139621](https://x.com/mitchellh/status/2090875695346139621)); that describes **tip/1.4.0-to-be**, not any current stable. Target stable 1.3.x behavior; treat animation as a post-1.4 upgrade.
- Quality history worth knowing: 1.2.0 (2025-09-15) fixed gamma blending of images; 1.3.0 (2026-03-09) fixed crashes with crafted large images and with rapid image cycling (Yazi-style) — i.e. transmit-heavy workloads were crashy before 1.3.0 ([1.3.0 release notes](https://ghostty.org/docs/install/release-notes/1-3-0)); old scroll-stick bug with `c/r`-scaled images fixed Dec 2024 (#2332).
- Detection: Ghostty sets `TERM=xterm-ghostty`, `TERM_PROGRAM=ghostty`, `GHOSTTY_RESOURCES_DIR`, and (like kitty) answers the `a=q` + DA1 probe. Note Incredible checks `GHOSTTY_SOCKET`/`KITTY_WINDOW_ID` too. Env vars don't survive SSH; the probe does.
- Where spec and Ghostty differ today: animation section of the spec (§1.8) — spec'd, kitty-implemented, Ghostty-stable-missing; relative placements likewise. Everything else we need matches the spec.

---

# 4. Recommended architecture for pacmantui

## 4.1 Bandwidth analysis

Classic Pac-Man screen: 224×288 px. Base64 inflates by 4/3. Per full frame:

| frame | raw f=32 | b64 f=32 | raw f=24 | b64 f=24 |
|-------|----------|----------|----------|----------|
| 224×288 (1×) | 258 KB | 344 KB (~84 chunks) | 194 KB | 258 KB |
| 448×576 (2×) | 1.03 MB | 1.38 MB | 774 KB | 1.03 MB |
| 672×864 (3×) | 2.32 MB | 3.10 MB | 1.74 MB | 2.32 MB |
| 896×1152 (4×) | 4.13 MB | 5.51 MB | 3.10 MB | 4.13 MB |

Per second: 2× @ 30 fps ≈ 31 MB/s (f=24), @ 60 fps ≈ 62 MB/s; 4× @ 60 fps ≈ 248 MB/s.
Ghostty's VT throughput is on the order of hundreds of MB/s locally, so 2×/30-60 fps raw
is comfortable; 4×/60 is pushing it. Two independent mitigations, both easy: (a)
**transmit at 1× and integer-upscale... no — see scaling note below**; (b) `o=z`: Pac-Man
frames are flat-color and deflate very well (expect ≥10×, i.e. a few tens of KB/frame
even at 4×), at the cost of ~1-2 ms/frame of compression. Start raw f=24; add `o=z` if
profiling says the tty write is the bottleneck.

Scaling note: letting the terminal scale 224×288 into a `c=`,`r=` cell box minimizes
bandwidth but surrenders the scaling filter to the terminal (GPU sampling; not guaranteed
nearest-neighbour → blurry or shimmering pixel art, plus non-integer scale factors from
arbitrary cell sizes). **Do the integer scaling yourself** (nearest-neighbour ×2/×3/×4 —
it's a memcpy-pattern loop, and it's exactly what makes `o=z` so effective since scaled
rows/pixels repeat) and place the image without `c=`/`r=` so pixels map 1:1.

## 4.2 Architecture choice

Three candidates:

1. **Cached sprite placements** — transmit each sprite once, then per frame issue `a=p` per moving actor (with `X/Y` sub-cell pixel offsets) and delete stale placements. Rejected: ~10+ graphics commands per frame with create/replace/delete churn; z-order bookkeeping across maze/dots/actors; `X/Y` offsets must stay < cell size so cell+offset math per actor per frame; historically the buggier path in terminals (Ghostty's pre-1.3 crash was exactly "rapid image cycling"). All to save bandwidth we don't need to save.
2. **Unicode placeholders** (Incredible's way) — beautiful for cell-aligned TUIs, wrong for us: positions quantize to whole cells and Pac-Man moves per-pixel (§2.3). Would also force id-in-fg-color text management for no benefit since we own the whole screen.
3. **Full framebuffer per frame** — compose everything in an in-memory pixel buffer (the classic emulator/PICO-8 model: maze layer + dots + sprites blitted in software), send it as **one image, one placement** per frame. One code path, no partial-update bugs, flicker controlled at exactly one point, trivially supports pixel-perfect movement, palettes, screen shake, whatever. **This is the recommendation**, with sprite-blitting done in our own Rust code, not the terminal's.

## 4.3 Recommended frame loop (exact sequences)

One-time setup:

```
\x1b[?1049h          enter alternate screen (auto-cleans placements on exit)
\x1b[?25l            hide cursor
raw mode             (crossterm)
\x1b_Gi=1,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\  \x1b[c     capability probe (§1.6)
ioctl TIOCGWINSZ     → cols, rows, xpixel, ypixel → cell_w, cell_h
scale = max integer k such that 224k ≤ xpixel and 288k ≤ ypixel (min 1)
anchor cell = top-left cell of the centered 224k×288k rectangle:
  col = ((xpixel − 224k)/2) / cell_w + 1,  row = ((ypixel − 288k)/2) / cell_h + 1
  (letterboxing = whatever background you paint on the text layer)
```

Per frame (double-buffered ids, e.g. `ID_A=42`, `ID_B=43`, alternating; placement `p=1`):

```
\x1b[?2026h                                   begin synchronized update
…optional text drawing (HUD/debug) anywhere…
\x1b[<row>;<col>H                             cursor to anchor cell
\x1b_Ga=t,f=24,s=<224k>,v=<288k>,i=<NEW>,q=2,m=1;<b64 4096>\x1b\
\x1b_Gm=1;<…>\x1b\ …  \x1b_Gm=0;<…>\x1b\      (add o=z before base64 if enabled)
\x1b_Ga=p,i=<NEW>,p=1,z=-1,C=1,q=2\x1b\       place under text, don't move cursor
\x1b_Ga=d,d=i,i=<OLD>,q=2\x1b\                drop the previous frame's placement
\x1b[?2026l                                   end synchronized update
swap NEW/OLD
```

Why this shape:

- **Two alternating ids** sidestep the spec rule that retransmitting an id deletes its placements first (a same-id scheme has a window where nothing is placed). The old frame's placement stays visible until the new one exists; the delete then removes it. Next frame's transmit into `OLD` overwrites its stored data, so quota usage stays ~2 frames.
- **Synchronized output (2026)** makes the whole transmit+place+delete present atomically in Ghostty regardless; keep it even though double-buffering alone usually suffices — it also covers the text layer. (Belt and braces: either alone is *probably* enough; both is cheap.)
- `z=-1` keeps the image under text so score/HUD can be plain (fast) text on top; use z ≥ 0 if you want the image over text instead.
- `q=2` everywhere: zero responses to drain from stdin during play.
- Write the whole frame's bytes into one buffer and `write()` it in one call (single flush per frame); build the base64 into a reusable buffer to avoid per-frame allocation.
- Timing: drive at 30 fps first (classic Pac-Man logic runs fine on a 60 Hz tick with 30 fps presentation in a terminal); measure before going 60.

## 4.4 Resize, cleanup, panic

- **SIGWINCH**: re-run the setup math (ioctl → scale/anchor), `\x1b_Ga=d,d=A\x1b\` + `\x1b[2J` (2J also deletes images per spec, but be explicit), repaint letterbox text, resume the loop. If the window gets too small for 1×, fall back to `c=`,`r=` terminal downscaling or a "window too small" text screen.
- **Exit path** (normal + panic hook + SIGINT/SIGTERM handler → same function, idempotent): `\x1b_Ga=d,d=A\x1b\` (free placements *and* data), `\x1b[?2026l` (in case we died mid-frame), `\x1b[?25h`, `\x1b[?1049l`, disable raw mode. Leaving the alt screen already clears alt-screen placements (§1.9), but the uppercase delete also releases the ~2 frames of stored data immediately. Install via a guard struct's `Drop` plus `std::panic::set_hook` chaining the previous hook (so the backtrace still prints onto a restored screen), plus a `signal_hook`-registered handler for SIGINT/SIGTERM.

## 4.5 Rust crates

- **Hand-roll the protocol** (recommendation): the entire needed surface is ~150 LOC (chunked b64 writer + 4 format strings + probe parser). Existing options evaluated on crates.io (2026-09-28): `kitty-graphics-protocol` 0.1.3 (2026-02, 234 downloads total — too young/unproven), `kitty_image` 0.1.0 (dormant since 2023-12), `ratatui-image` 11.x/12.0.0-rc (healthy and actively maintained, but designed as a ratatui widget for static images — wrong shape for a 30-60 fps framebuffer, and drags in the ratatui stack). None earns a dependency for this use.
- **crossterm 0.29** — raw mode, input events (keyboard incl. kitty keyboard protocol enhancement flags, resize events), alt screen, cursor. Its `terminal::window_size()` already returns `WindowSize { rows, columns, width, height }` from TIOCGWINSZ, so no manual `libc::ioctl` needed (keep `rustix`/`libc` in mind only if we drop crossterm).
- **base64 0.23** — the standard encoder; use `EncodeSliceOn`-style APIs (`encode_slice` into a reused buffer).
- **image 0.25** (`default-features = false, features = ["png"]`) — decode sprite-sheet PNGs at build/startup, exactly as incredible-flappy does (`assets.rs`); or skip PNG entirely and embed sprites as const arrays.
- **flate2 1.x** — only if/when enabling `o=z`.
- **signal-hook 0.3** (or crossterm's event stream for resize + minimal `libc` for signals) — SIGWINCH/SIGINT/SIGTERM.

## 4.6 Open risks

1. **Ghostty stable vs tip drift**: we target 1.3.x semantics; when 1.4.0 lands (animation, relative placements, "100% complete"), nothing we rely on changes, but retest — image-path regressions have happened at each minor.
2. The terminal's scaling filter is unspecified — avoided by integer self-scaling, but the "window smaller than 1×" fallback will hit it; acceptable for a degraded mode.
3. Throughput at 4×/60 fps without `o=z` may contend with Ghostty's parser on slower machines; `o=z` is the planned relief valve. Measure with a real frame-time HUD early.
4. Article's claim that Flappy "hit the limits of the protocol in two places" is unexplained (framework repo private); nothing in the public code identifies them. Most plausible candidates given our capture — cell-quantized placeholder positioning and per-screen retransmission cost — are both avoided by our architecture, but keep an eye out.
5. Running inside tmux/screen would break everything (APC passthrough, placeholder quirks); explicitly detect and refuse/fallback rather than glitch.
