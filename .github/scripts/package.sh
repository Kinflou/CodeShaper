#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."

servers=$(realpath "$1")
mkdir -p "$2"
packages=$(realpath "$2")
version=$3

flags=()
if [ "${PRERELEASE:-true}" = true ]; then flags+=(--pre-release); fi

trap 'rm -rf editors/vscode/server editors/vscode/LICENSE' EXIT
cp LICENSE editors/vscode/LICENSE

for directory in "$servers"/server-*/; do
    [ -d "$directory" ] || { echo "No platform was built" >&2; exit 1; }
    target=$(basename "$directory")
    target=${target#server-}

    rm -rf editors/vscode/server
    mkdir editors/vscode/server
    cp "$directory"codeshaper* editors/vscode/server/
    chmod +x editors/vscode/server/codeshaper*

    (
        cd editors/vscode
        npx --no-install vsce package "$version" --no-git-tag-version --no-update-package-json \
            --target "$target" "${flags[@]}" --out "$packages/codeshaper-$target.vsix"
    )

    case "$target" in
        win32-*) (cd editors/vscode/server && zip -q "$packages/codeshaper-$target.zip" codeshaper.exe) ;;
        *) tar -czf "$packages/codeshaper-$target.tar.gz" -C editors/vscode/server codeshaper ;;
    esac
done

(cd "$packages" && sha256sum codeshaper-* > SHA256SUMS)
