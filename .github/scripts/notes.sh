#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."

packages=$1
tag=$2
pattern=$3

previous=$(git tag --list "$pattern" --merged HEAD --sort=-version:refname | awk -v tag="$tag" '$0 != tag && !found { found = 1; print }')

if [ -n "$previous" ]; then
    echo "## Changes since $previous"
    range=("$previous..HEAD")
else
    echo "## Latest changes"
    range=(--max-count=20 HEAD)
fi

echo
git log --no-merges --format='- %s (%h)' "${range[@]}"

missing=()

echo
echo "## Install"
echo
echo "| Platform | Extension with server | Command line only |"
echo "| --- | --- | --- |"

for platform in "linux-x64:Linux x64:tar.gz" "linux-arm64:Linux arm64:tar.gz" "win32-x64:Windows x64:zip" "darwin-arm64:macOS arm64:tar.gz"; do
    IFS=: read -r target name archive <<< "$platform"

    if [ -f "$packages/codeshaper-$target.vsix" ]; then
        echo "| $name | \`codeshaper-$target.vsix\` | \`codeshaper-$target.$archive\` |"
    else
        missing+=("$name")
    fi
done

echo
echo "Run *Extensions: Install from VSIX…*, or \`code --install-extension codeshaper-linux-x64.vsix\`. The server is included, and it doesn't update itself."

if [ ${#missing[@]} -gt 0 ]; then
    echo
    echo "**Left out because the build or its tests failed:** $(IFS=,; echo "${missing[*]}" | sed 's/,/, /g')."
fi
