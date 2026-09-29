# Validation 00 — Kitty graphics smoke test

Date: 2026-09-28 · Environment: Ghostty 1.3.2-HEAD-+4a0e9e1 embedded in cmux.app,
macOS Darwin 27.0.0, TERM=xterm-256color, TERM_PROGRAM=ghostty.

## What was tested

A Python script (scratchpad `kitty_smoke.py`) run in a real interactive Ghostty
pane (cmux workspace:1, surface:2):

1. Kitty graphics support query: `ESC _G a=q,i=31337,f=24,s=1,v=1 ... ESC \`
   with 1x1 RGB payload, reading the tty response in raw mode.
2. Direct transmission + display of a 64x64 RGB (f=24, t=d implied) red/yellow
   checkerboard via `a=T`, base64, 4096-byte chunking.

## Results

- Query response: `\x1b_Gi=31337;OK\x1b\\` → **protocol supported**.
- Checkerboard **visibly rendered as pixel graphics** in the pane.
- Pixel screenshot captured of the (unfocused) cmux window via
  `screencapture -x -l <windowid>`, window id found with a Swift
  CGWindowListCopyWindowInfo helper.

Evidence: `evidence/00-kitty-smoke-checkerboard.png` (right pane shows the
query response text and the rendered checkerboard).

## Conclusions

- Real pixel graphics via kitty protocol work in this exact environment.
- Screenshot pipeline for acceptance evidence: window-id `screencapture`
  (cmux-cua computer-use tools were not onboarded at test time).
- `cmux read-screen` is text-only; never usable as graphics proof.
