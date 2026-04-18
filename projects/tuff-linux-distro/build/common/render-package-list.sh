#!/bin/bash
set -euo pipefail

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
source "${DISTRO_DIR}/build/common/setup-cargo-path.sh"
TOOLS_MANIFEST="${DISTRO_DIR}/tools/tuff-distro-tools/Cargo.toml"
TOOLS_TARGET_DIR="${DISTRO_DIR}/out/cargo-tools-target"

if ! command -v cargo >/dev/null 2>&1; then
    echo "[ERROR] cargo is required to run render-package-list." >&2
    exit 1
fi

exec env CARGO_TARGET_DIR="${TOOLS_TARGET_DIR}" cargo run --quiet --manifest-path "${TOOLS_MANIFEST}" -- render-package-list "$@"
