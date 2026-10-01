#!/usr/bin/env bash
# Fails when tracked text files contain Portuguese accented letters: English is the
# project language (CLAUDE.md). A line that must keep one carries "check-english:allow".
# Skipped: docs/pt-br/ (the translation) and NOTICE (third-party author names).
set -euo pipefail
cd "$(dirname "$0")/.."

hits=$(git grep -nI '' -- . ':(exclude)docs/pt-br/' ':(exclude)NOTICE' |
  perl -CSD -Mutf8 -ne '
    next if /check-english:allow/;
    my ($text) = /^[^:]+:\d+:(.*)/s or next;
    print if $text =~ /[áàâãéêíóôõúçÁÀÂÃÉÊÍÓÔÕÚÇ]/; # check-english:allow
  ' || true)

if [ -z "$hits" ]; then
  echo "check-english: ok"
  exit 0
fi
echo "$hits"
echo
echo "check-english: Portuguese text found"
exit 1
