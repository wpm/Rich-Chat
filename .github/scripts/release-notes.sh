#!/usr/bin/env bash
# The top of an app release's notes: how to install each file, so that
# the list of assets under it needs no explaining. GitHub appends the
# generated list of changes to it.
#
#   release-notes.sh <version>
#
# Environment:
#   MACOS_SIGNED      "true" when the macOS build will be signed
#   MACOS_NOTARIZED   "true" when it will be notarized as well
set -euo pipefail

version=${1:?usage: release-notes.sh <version>}

cat <<NOTES
Rich Chat $version for the desktop: a chat window with rich formatting, Markdown, LaTeX math, and syntax-highlighted code, rendered as you type. The same app runs in the browser at <https://wpm.github.io/Rich-Chat/> with nothing installed.

| Platform | Download | Then |
| --- | --- | --- |
| macOS (Apple silicon and Intel) | the \`.dmg\` | Open it and drag Rich Chat to Applications. |
| Windows | the \`-setup.exe\` (or the \`.msi\`) | Run it. |
| Linux | the \`.deb\`, the \`.rpm\`, or the \`.AppImage\` | Install the package, or mark the AppImage executable and run it. |
NOTES

if [ "${MACOS_NOTARIZED:-}" = true ]; then
  :
elif [ "${MACOS_SIGNED:-}" = true ]; then
  cat <<'NOTES'

The macOS app is signed but not notarized, so the first time it opens, macOS asks to confirm.
NOTES
else
  cat <<'NOTES'

The macOS app is not signed, so macOS refuses to open it and says it is damaged. It is not; clear the quarantine mark once and it opens normally:

```sh
xattr -d com.apple.quarantine "/Applications/Rich Chat.app"
```
NOTES
fi

cat <<'NOTES'

Windows may show a SmartScreen warning for a new installer; choose "More info" and "Run anyway".
NOTES
