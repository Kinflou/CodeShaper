#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."

version=$(node -p 'require("./editors/vscode/package.json").version')
commit=$(git rev-parse --short=7 HEAD)

if [ "${GITHUB_REF_TYPE:-}" = tag ]; then
    cli=$(sed -n 's/^version = "\(.*\)"$/\1/p' cli/Cargo.toml | head -1)

    if [ "$GITHUB_REF_NAME" != "v$version" ] || [ "$version" != "$cli" ]; then
        echo "Tag $GITHUB_REF_NAME needs version ${GITHUB_REF_NAME#v} in both, but the extension has $version and the CLI has $cli" >&2
        exit 1
    fi
else
    version="${version%.*}.${GITHUB_RUN_NUMBER:-0}"
fi

echo "version=$version"
echo "label=$version ($commit)"
