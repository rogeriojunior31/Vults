#!/usr/bin/env bash
# Plays a recorded Claude Code stream in place of the real CLI, from its working folder: reads the
# user's message first, as Claude Code does, then prints ./cassette.jsonl, and after each permission
# request reads the app's answer and keeps it in ./decisions for the eval to check.
set -u
IFS= read -r -t 10 _message
# The cassette on fd 3: stdin stays the app's, where its answers come. A last line without a
# newline still counts.
while IFS= read -r line <&3 || [[ -n $line ]]; do
  printf '%s\n' "$line"
  case $line in
    '{"type":"control_request"'*) IFS= read -r -t 10 answer && printf '%s\n' "$answer" >>decisions ;;
  esac
done 3<cassette.jsonl
