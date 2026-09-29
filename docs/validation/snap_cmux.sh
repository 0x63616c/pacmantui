#!/bin/bash
# Fresh-pixel capture of the cmux window: activate briefly, capture, restore focus.
set -e
out="$1"
SCRATCH="/private/tmp/claude-501/-Users-calum-code-github-com-0x63616c-pacmantui/c15dbe8f-3e78-4ce0-913b-dbec9b4bde26/scratchpad"
prev=$(osascript -e 'tell application "System Events" to get bundle identifier of first process whose frontmost is true')
osascript -e 'tell application id "com.cmuxterm.app" to activate'
sleep ${SNAP_DELAY:-0.7}
winid=$("$SCRATCH/winid_main")
screencapture -x -l "$winid" "$out"
if [ "$prev" != "com.cmuxterm.app" ]; then
  osascript -e "tell application id \"$prev\" to activate"
fi
echo "captured $out (winid $winid, restored $prev)"
