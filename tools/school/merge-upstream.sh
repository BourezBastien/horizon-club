#!/usr/bin/env bash
# Bring Horizon's engine into the club's branch, keeping what is the fork's own.
#
#   tools/school/merge-upstream.sh [branch]        (branch of the `upstream` remote, main by default)
#
# Horizon's launcher, workflows and tools change every day and are not this edition's: after the
# merge they are put back exactly as this branch has them, as is this edition's README. What
# remains to resolve by hand is the engine's conflicts, expected only around the `managed`
# feature (crates/horizon-client/src/managed.rs and its call sites). A merge attribute cannot do
# this: it settles two versions of one file, not a file upstream adds to a folder the fork has
# let go of.
#
# Commit or stash everything first: what no branch tracks is removed under the fork's folders.
# Ignored files (build folders) stay.
set -euo pipefail

cd "$(dirname "$0")/../.."
branch=${1:-main}
own=(launcher .github docs/school-edition tools/school README.md)
checks=(
    'cargo check -p horizon-game --no-default-features --features audio,controllers,managed'
    'cargo test -p horizon-client-lib --lib managed::'
    'cargo check --manifest-path launcher/Cargo.toml --all-targets'
)

git fetch upstream
if ! git merge --no-commit --no-ff -X no-renames "upstream/$branch"; then
    # The fork's folders go back to what this branch has: upstream's changes to them never land.
    git rm -r -q --cached -- "${own[@]}" >/dev/null 2>&1 || true
    git checkout HEAD -- "${own[@]}"
    git clean -fdq -- "${own[@]}"
fi

left=$(git status --short | grep -E '^(UU|AA|DU|UD|AU|UA|DD) ' || true)
if [ -n "$left" ]; then
    printf 'Conflicts left in the engine:\n%s\n' "$left"
    printf 'Resolve them, then run:\n'
    printf '  %s\n' "${checks[@]}"
    printf 'and commit the merge (the launcher does not follow upstream: nothing of it is lost).\n'
    exit 1
fi

printf 'Merged with no conflict in the engine. Before committing, run:\n'
printf '  %s\n' "${checks[@]}"
