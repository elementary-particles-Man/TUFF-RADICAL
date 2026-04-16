#!/bin/bash
set -euo pipefail

usage() {
    cat <<'EOF'
usage: render-package-list.sh [--format lines|comma] [package-group...]

Render one or more package manifests from projects/tuff-linux-distro/packages/.
The default output format is newline-delimited package names.
EOF
}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
format="lines"
groups=()

while (($# > 0)); do
    case "$1" in
        --format)
            shift
            format="${1:-}"
            if [[ -z "$format" ]]; then
                echo "[ERROR] --format requires a value" >&2
                exit 1
            fi
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            groups+=("$1")
            ;;
    esac
    shift
done

if [[ ${#groups[@]} -eq 0 ]]; then
    groups=("tuff-base")
fi

files=()
for group in "${groups[@]}"; do
    manifest="${DISTRO_DIR}/packages/${group}/manifest.txt"
    if [[ ! -f "$manifest" ]]; then
        echo "[ERROR] Missing package manifest: $manifest" >&2
        exit 1
    fi
    files+=("$manifest")
done

render_lines() {
    awk '
        {
            sub(/[[:space:]]*#.*/, "", $0)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0)
            if ($0 != "" && !seen[$0]++) {
                print $0
            }
        }
    ' "${files[@]}"
}

case "$format" in
    lines)
        render_lines
        ;;
    comma)
        render_lines | paste -sd, -
        ;;
    *)
        echo "[ERROR] Unsupported format: $format" >&2
        exit 1
        ;;
esac
