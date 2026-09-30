#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Create a production release through GitHub Actions.

Usage:
  scripts/release.sh VERSION [--dry-run]

Examples:
  scripts/release.sh 0.2.0
  scripts/release.sh 0.2.0-rc.1 --dry-run
EOF
}

version=
dry_run=false
for argument in "$@"; do
    case "$argument" in
        --dry-run) dry_run=true ;;
        -h|--help) usage; exit 0 ;;
        -*)
            echo "error: unsupported option: $argument" >&2
            usage >&2
            exit 2
            ;;
        "")
            echo "error: version cannot be empty" >&2
            exit 2
            ;;
        *)
            [ -z "$version" ] || {
                echo "error: only one version is allowed" >&2
                exit 2
            }
            version=$argument
            ;;
    esac
done

[ -n "$version" ] || {
    echo "error: VERSION is required" >&2
    usage >&2
    exit 2
}

version=${version#v}
printf '%s\n' "$version" | grep -Eq \
    '^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$' || {
    echo "error: version must look like 1.2.3 or 1.2.3-rc.1" >&2
    exit 2
}

command -v gh >/dev/null 2>&1 || {
    echo "error: GitHub CLI (gh) is required" >&2
    exit 1
}
branch=$(git branch --show-current)
[ "$branch" = "main" ] || {
    echo "error: releases must start from main, current branch is $branch" >&2
    exit 1
}

git diff --quiet || {
    echo "error: working tree has unstaged changes" >&2
    exit 1
}
git diff --cached --quiet || {
    echo "error: index has staged changes" >&2
    exit 1
}

remote_branch=$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)
[ "$remote_branch" = "origin/main" ] || {
    echo "error: main must track origin/main" >&2
    exit 1
}
git fetch origin main
git diff --quiet HEAD origin/main || {
    echo "error: local main is not synchronized with origin/main" >&2
    exit 1
}

if [ "$dry_run" = true ]; then
    echo "Dry run: would trigger Release workflow for v$version"
    exit 0
fi

echo "Starting GitHub Actions Release workflow for v$version..."
gh workflow run release.yml --ref main --field "version=$version"

run_id=
attempt=0
while [ -z "$run_id" ] && [ "$attempt" -lt 30 ]; do
    run_id=$(gh run list \
        --workflow release.yml \
        --branch main \
        --limit 10 \
        --json databaseId,event,status,headBranch \
        --jq 'map(select(.event == "workflow_dispatch" and (.status == "queued" or .status == "in_progress"))) | .[0].databaseId // empty')
    [ -n "$run_id" ] || {
        attempt=$((attempt + 1))
        sleep 2
    }
done

[ -n "$run_id" ] || {
    echo "error: could not find the started Release workflow run" >&2
    exit 1
}

echo "Watching workflow run $run_id..."
gh run watch "$run_id" --exit-status
gh release view "v$version" --json url --jq '.url'
