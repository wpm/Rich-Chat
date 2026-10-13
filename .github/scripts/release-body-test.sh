#!/usr/bin/env bash
# Tests for release-body.sh, run by ci.yml.
set -euo pipefail
cd "$(dirname "$0")"

failures=0
check() { # <description> <condition>...
  local what=$1; shift
  if "$@"; then echo "ok: $what"; else echo "FAIL: $what" >&2; failures=$((failures + 1)); fi
}

generated=$'## What\'s Changed\n* A change by @someone'
notes=$(./release-notes.sh 1.2.3)

# The notes go on top of what GitHub generated, with a blank line between.
body=$(./release-body.sh 1.2.3 <<< "$generated")
check "notes come first" test "$body" = "$notes"$'\n\n'"$generated"

# A rerun finds the notes already there and leaves the body alone.
again=$(./release-body.sh 1.2.3 <<< "$body")
check "a rerun changes nothing" test "$again" = "$body"

# An empty body, notes not generated, is just the notes.
empty=$(./release-body.sh 1.2.3 < /dev/null)
check "an empty body gets the notes alone" test "$empty" = "$notes"

# The macOS signing status follows the environment, as in release-notes.sh.
signed=$(MACOS_SIGNED=true ./release-body.sh 1.2.3 <<< "$generated")
check "the signing status is passed on" grep -q "signed but not notarized" <<< "$signed"

[ "$failures" -eq 0 ]
