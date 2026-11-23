#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."

packages=$1
tag=$2
title=$3
pattern=$4
shift 4

if [ ! -f "$packages/codeshaper-linux-x64.vsix" ]; then
    echo "Nothing is published without Linux x64" >&2
    exit 1
fi

notes=$(mktemp)
.github/scripts/notes.sh "$packages" "$tag" "$pattern" > "$notes"

if gh release view "$tag" > /dev/null 2>&1; then
    gh release upload "$tag" "$packages"/* --clobber
    gh release edit "$tag" --notes-file "$notes"
else
    gh release create "$tag" "$packages"/* --title "$title" --notes-file "$notes" "$@"
fi
