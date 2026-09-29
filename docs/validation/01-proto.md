# Validation 01 — rendering prototype (src/bin/proto.rs)

Date: 2026-09-28 · Environment: Ghostty 1.3.2 in cmux, macOS. Run in cmux
workspace:1/surface:2 (real interactive pane), driven via `cmux send`,
captured via focus-flip `screencapture -l <winid>` helper.

## Verified

- Animated pixel graphics: 60 fps loop (frame counter vs wall clock), moving
  disc with mouth animation over wall/pellet scene. Evidence:
  `evidence/01-proto-4x.png` (fresh-pixel capture).
- Responsive input: arrow keys steer (observed position/direction change),
  `p` pause, `q` quit.
- Opaque #000000 background over whole pane incl. letterbox.
- Resize: splitting the pane live rescaled 4x→3x without losing state;
  closing the split returned to 4x. No crash, no residue.
- Clean exit: `q` → alt screen left, prompt restored, process exited,
  images deleted (a=d,d=A), cursor shown.
- Performance (o=z zlib + f=24 + double-buffered ids + sync output):
  4x (896x1152): draw avg ~1.4-3.2 ms, max ~10 ms, ~37 KB/frame.
  3x: ~1.1 ms, 25 KB/frame. Without o=z at 6x: 29-31 ms/frame and PTY
  saturation stalls when the pane is occluded (Ghostty throttles hidden
  panes) — o=z is therefore REQUIRED, not optional.

## Environment gotchas recorded

- Occluded panes: Ghostty pauses presentation; `screencapture -l` then
  returns stale pixels. Use scratchpad `snap_cmux.sh` (activate cmux →
  capture → restore previous app).
- Any kitty command without `q=2` leaks responses into the shell input.
- `cmux send-key <letter>` silently no-ops; letters must go via `cmux send`.
  Arrows/ctrl+keys work with send-key.

## Architecture confirmed for the game renderer

Per docs/research/rendering.md §4: full framebuffer, self integer-scaling
(cap 4x), f=24 + o=z, 4096-byte chunks, alternating ids 42/43, place p=1
z=-1 C=1, delete old placement d=i, all inside CSI ?2026 brackets, single
write per frame. Scale recompute + repaint on Resize event. Cleanup guard +
panic hook restore terminal and delete images.
