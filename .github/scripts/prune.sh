#!/bin/bash
set -euo pipefail

keep=$1

tags=$(gh release list --limit 1000 --json tagName,isPrerelease,isDraft \
    --jq '.[] | select(.isPrerelease and (.isDraft | not) and (.tagName | test("^dev-[0-9]+$"))) | .tagName')

sort -t- -k2,2nr <<< "$tags" | tail -n "+$((keep + 1))" | while read -r tag; do
    gh release delete "$tag" --cleanup-tag --yes
done
