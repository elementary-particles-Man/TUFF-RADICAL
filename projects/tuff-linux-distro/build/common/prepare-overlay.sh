#!/bin/bash
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
source "${DISTRO_DIR}/build/common/setup-cargo-path.sh"
TOOLS_MANIFEST="${DISTRO_DIR}/tools/tuff-distro-tools/Cargo.toml"
TOOLS_TARGET_DIR="${DISTRO_DIR}/out/cargo-tools-target"
TOOLS_BIN="${TOOLS_TARGET_DIR}/release/tuff-distro-tools"
STAGE_DIR="${DISTRO_DIR}/out/overlay-stage"

if [ ! -f "${TOOLS_MANIFEST}" ]; then
    echo "[ERROR] Missing Rust tool manifest: ${TOOLS_MANIFEST}" >&2
    exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "[ERROR] cargo is required to build tuff-distro-tools." >&2
    exit 1
fi

mkdir -p "${TOOLS_TARGET_DIR}"
CARGO_TARGET_DIR="${TOOLS_TARGET_DIR}" cargo build --quiet --release --manifest-path "${TOOLS_MANIFEST}"

rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}"
cp -a "${DISTRO_DIR}/overlay/." "${STAGE_DIR}/"
install -D -m 0755 "${TOOLS_BIN}" "${STAGE_DIR}/usr/local/lib/tuff/tuff-distro-tools"

printf '%s\n' "${STAGE_DIR}"
