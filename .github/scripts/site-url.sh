#!/usr/bin/env bash
# Where the site, or a directory of it, is served from.
#
#   site-url.sh [<directory>]
#
# Writes two step outputs: url, the full address, and path, the address
# without its origin, which is what `trunk build --public-url` takes. The
# site's address comes from the Pages settings; before Pages is enabled it
# is the default one. <directory> is appended, as a directory.
#
#   site-url.sh          # url=https://wpm.github.io/Rich-Chat/  path=/Rich-Chat/
#   site-url.sh pr/12    # url=https://wpm.github.io/Rich-Chat/pr/12/  path=/Rich-Chat/pr/12/
#
# Environment:
#   GITHUB_REPOSITORY, and gh with a token   the repository and its settings
#   GITHUB_OUTPUT                            where the outputs go (stdout without it)
set -euo pipefail

repo=${GITHUB_REPOSITORY:?}
site=$(gh api "repos/$repo/pages" --jq .html_url 2>/dev/null) \
  || site="https://${repo%%/*}.github.io/${repo#*/}/"
url="${site%/}/${1:+${1%/}/}"
path=${url#*://}
path=/${path#*/}

{
  echo "url=$url"
  echo "path=$path"
} >> "${GITHUB_OUTPUT:-/dev/stdout}"
