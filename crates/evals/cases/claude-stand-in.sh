#!/usr/bin/env bash
# Plays a recorded Claude Code stream (VULTS_EVAL_CASSETTE) in place of the real CLI: reads the
# user's message first, as Claude Code does, then prints the cassette, and after each permission
# request reads the app's answer and keeps it (VULTS_EVAL_DECISIONS) for the eval to check.
set -u
IFS= read -r _message
# The cassette on fd 3: stdin stays the app's, where its answers come.
while IFS= read -r line <&3; do
  printf '%s\n' "$line"
  case $line in
    *'"control_request"'*) IFS= read -r answer && printf '%s\n' "$answer" >>"$VULTS_EVAL_DECISIONS" ;;
  esac
done 3<"$VULTS_EVAL_CASSETTE"
