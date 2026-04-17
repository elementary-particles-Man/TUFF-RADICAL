#!/bin/bash
# TUFF-RADICAL: Surgical Package List Renderer v2 (Consistent Logos)
set -euo pipefail

usage() {
    cat <<'EOF'
usage: render-package-list.sh [--format lines|comma] [package-group...]

Render one or more package lists from projects/tuff-linux-distro/packages/.
Looks for *.txt files in each package directory.
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

# Collect all .txt package lists in the specified group directories
files=()
for group in "${groups[@]}"; do
    group_dir="${DISTRO_DIR}/packages/${group}"
    if [[ ! -d "$group_dir" ]]; then
        echo "[ERROR] Missing package directory: $group_dir" >&2
        exit 1
    fi
    # Only pick .txt files, ignoring README/MANIFEST
    mapfile -t group_files < <(ls "${group_dir}"/*.txt 2>/dev/null)
    files+=("${group_files[@]}")
done

if [[ ${#files[@]} -eq 0 ]]; then
    echo "[ERROR] No package lists (*.txt) found in: ${groups[*]}" >&2
    exit 1
fi

render_lines() {
    # Enhanced parsing: remove comments, trim whitespace, remove duplicates
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
        render_lines | tr '\n' ',' | sed 's/,$//'
        ;;
    *)
        echo "[ERROR] Unsupported format: $format" >&2
        exit 1
        ;;
esac
