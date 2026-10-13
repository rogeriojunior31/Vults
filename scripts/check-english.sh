#!/usr/bin/env bash
# Fails when tracked text files contain Portuguese accented letters: English is the
# project language (CLAUDE.md). A line that must keep one carries "check-english:allow".
# Skipped: docs/pt-br/ and README.pt-br.md (the translation), docs/site.json (the website's summary in each language),
# the app's translations (ui/src/i18n/, crates/core/src/i18n.rs), NOTICE (third-party author names) and
# design/explorations/ (the owner's prototypes, kept verbatim as the design reference).
# The same reference exception applies to the owner's newer design studies below.
set -euo pipefail
cd "$(dirname "$0")/.."

hits=$(git grep -nI '' -- . ':(exclude)docs/pt-br/' ':(exclude)README.pt-br.md' ':(exclude)docs/site.json' ':(exclude)ui/src/i18n/' ':(exclude)crates/core/src/i18n.rs' ':(exclude)NOTICE' ':(exclude)design/explorations/' ':(exclude)design/compact-nest-review/' ':(exclude)design/connections-local/' ':(exclude)design/vults-living-flock.html' |
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
