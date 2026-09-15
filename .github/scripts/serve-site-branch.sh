#!/usr/bin/env bash
# Points GitHub Pages at the branch the site is published to, enabling
# Pages if it is not yet. Pages has to serve the branch as it is, rather
# than one workflow's artifact, for pull request previews to share the
# site with the root. Does nothing when already set up. Needs gh with a
# token allowed to write the Pages settings.
set -euo pipefail

branch=${SITE_BRANCH:-gh-pages}
repo=${GITHUB_REPOSITORY:?}
wanted="legacy $branch /"

current=$(gh api "repos/$repo/pages" --jq '"\(.build_type) \(.source.branch) \(.source.path)"' 2>/dev/null || echo none)
if [ "$current" = "$wanted" ]; then
  echo "Pages serves the $branch branch."
else
  if [ "$current" = none ]; then method=POST; else method=PUT; fi
  if printf '{"build_type":"legacy","source":{"branch":"%s","path":"/"}}' "$branch" \
      | gh api -X "$method" "repos/$repo/pages" --input - > /dev/null; then
    echo "Pages now serves the $branch branch (was: $current)."
  else
    echo "::error::Could not point Pages at the $branch branch. Once, by hand: Settings > Pages > Build and deployment > Source: Deploy from a branch, $branch, / (root)."
    exit 1
  fi
fi
echo "Site: $(gh api "repos/$repo/pages" --jq .html_url)"
