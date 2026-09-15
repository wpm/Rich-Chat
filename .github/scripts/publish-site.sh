#!/usr/bin/env bash
# Publishes part of the website to the branch GitHub Pages serves.
#
# The branch holds the whole site: the app built from main at the root,
# and one preview per open pull request under pr/<number>/. Each call
# replaces exactly one of those and leaves the rest alone, then pushes
# the branch as a single fresh commit, so the repository never grows
# with old builds. A push that loses a race with another run is retried
# on top of what that run published.
#
#   publish-site.sh <path> [<directory>]
#
#   publish-site.sh . app/dist        # the root; pr/ is kept
#   publish-site.sh pr/12 app/dist    # a preview
#   publish-site.sh pr/12             # no directory: remove it
#
# Environment:
#   GITHUB_REPOSITORY, GITHUB_TOKEN   where to push (or SITE_REMOTE, a URL)
#   SITE_BRANCH                       the branch, gh-pages by default
#   KEEP_PREVIEWS                     with path ".": the pull request
#                                     numbers whose previews to keep;
#                                     every other pr/<number>/ is removed
set -euo pipefail

path=${1:?usage: publish-site.sh <path> [<directory>]}
source=${2:-}
branch=${SITE_BRANCH:-gh-pages}
remote=${SITE_REMOTE:-"https://x-access-token:${GITHUB_TOKEN:?}@github.com/${GITHUB_REPOSITORY:?}.git"}

if [ -n "$source" ] && [ ! -d "$source" ]; then
  echo "not a directory: $source" >&2
  exit 1
fi
source=${source:+$(cd "$source" && pwd)}

case "$path" in
  .) what="the site";;
  pr/*) what="preview $path";;
  *) echo "path must be . or pr/<number>, not $path" >&2; exit 1;;
esac
if [ -n "$source" ]; then
  message="Publish $what"
else
  message="Remove $what"
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for attempt in 1 2 3 4 5; do
  rm -rf "$work"
  git init -q -b "$branch" "$work"
  git -C "$work" remote add origin "$remote"

  # The branch's current tree, and the commit the push must replace.
  # An empty expectation tells force-with-lease the branch must not exist.
  if git -C "$work" fetch -q --depth=1 origin "$branch" 2>/dev/null; then
    expected=$(git -C "$work" rev-parse FETCH_HEAD)
    git -C "$work" restore -q --source=FETCH_HEAD --staged --worktree -- .
  else
    expected=""
  fi

  if [ "$path" = . ]; then
    find "$work" -mindepth 1 -maxdepth 1 ! -name .git ! -name pr -exec rm -rf {} +
    if [ -n "${KEEP_PREVIEWS+set}" ] && [ -d "$work/pr" ]; then
      for preview in "$work"/pr/*/; do
        [ -d "$preview" ] || continue
        number=$(basename "$preview")
        case " $KEEP_PREVIEWS " in
          *" $number "*) ;;
          *) echo "removing stale preview pr/$number"; rm -rf "$preview";;
        esac
      done
    fi
    cp -R "$source"/. "$work"/
  else
    rm -rf "${work:?}/$path"
    if [ -n "$source" ]; then
      mkdir -p "$work/$path"
      cp -R "$source"/. "$work/$path"/
    fi
  fi
  # Served as it is, without Jekyll.
  touch "$work/.nojekyll"

  git -C "$work" add -A
  if [ -n "$expected" ] && git -C "$work" diff --cached --quiet "$expected"; then
    echo "nothing to publish: $branch already has $what as it should be"
    exit 0
  fi
  git -C "$work" \
    -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit -q -m "$message"

  if git -C "$work" push -q --force-with-lease="$branch:$expected" origin "HEAD:$branch"; then
    echo "published: $message"
    exit 0
  fi
  echo "push rejected, another run published first; retrying ($attempt)" >&2
  sleep $((attempt * 2))
done

echo "gave up publishing $what after 5 attempts" >&2
exit 1
