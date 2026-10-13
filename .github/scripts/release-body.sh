#!/usr/bin/env bash
# An app release's body with the notes from release-notes.sh on top.
#
#   gh release view <tag> --json body --jq .body | release-body.sh <version>
#
# Reads the body the release has, usually what GitHub's Generate
# release notes button wrote, and prints it with the notes above it. A
# body that already starts with them is printed as it is, so rerunning
# the workflow does not stack a second copy.
#
# Environment: as release-notes.sh, which says how the macOS build is signed.
set -euo pipefail

version=${1:?usage: release-body.sh <version>}
notes=$("$(dirname "$0")/release-notes.sh" "$version")
body=$(cat)

if [ "${body#"$notes"}" != "$body" ]; then
  printf '%s\n' "$body"
elif [ -z "$body" ]; then
  printf '%s\n' "$notes"
else
  printf '%s\n\n%s\n' "$notes" "$body"
fi
