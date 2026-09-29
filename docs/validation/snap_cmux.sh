#!/bin/bash
# Fresh-pixel capture of the cmux window: activate briefly, capture, restore focus.
# (Occluded Ghostty panes freeze their pixels, so captures without the focus
# flip return stale frames.) The CGWindowID helper is compiled on demand from
# winid_main.swift next to this script.
set -e
out="$1"
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/winid_main.swift"
bin="${TMPDIR:-/tmp}/pacmantui-winid"
if [ ! -x "$bin" ] || [ "$src" -nt "$bin" ]; then
  swiftc -O -o "$bin" "$src"
fi
prev=$(osascript -e 'tell application "System Events" to get bundle identifier of first process whose frontmost is true')
osascript -e 'tell application id "com.cmuxterm.app" to activate'
sleep ${SNAP_DELAY:-0.7}
winid=$("$bin")
screencapture -x -l "$winid" "$out"
if [ "$prev" != "com.cmuxterm.app" ]; then
  osascript -e "tell application id \"$prev\" to activate"
fi
echo "captured $out (winid $winid, restored $prev)"
