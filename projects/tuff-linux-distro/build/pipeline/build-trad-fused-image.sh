#!/bin/bash
# TUFF-RADICAL: Surgical Fused Image Builder v4 (Atomic Merger)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PREPARE_OVERLAY="${DISTRO_DIR}/build/common/prepare-overlay.sh"
WORK_DIR="/tmp/tuff-fuse-work"
BASE_TAR="${DISTRO_DIR}/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar"
OUT_TAR="${DISTRO_DIR}/out/pipeline/tuff-fused-rootfs.tar"

cleanup() {
    echo "[INFO] Cleaning up fused work directory..."
    rm -rf "$WORK_DIR"
}

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] Root privileges required for atomic merge."
    exit 1
fi

if [ ! -f "$BASE_TAR" ]; then
    echo "[ERROR] Base RootFS tarball missing: $BASE_TAR"
    exit 1
fi

echo "--- Fusing TUFF-RADICAL Layers ---"
trap cleanup EXIT

mkdir -p "$WORK_DIR/merged"
mkdir -p "$(dirname "$OUT_TAR")"
OVERLAY_STAGE="$("${PREPARE_OVERLAY}")"

# Layer 1: Base System (Atomic Extract)
tar --numeric-owner -xf "$BASE_TAR" -C "$WORK_DIR/merged"

# Layer 2: Overlay Sync (Force Refresh)
cp -a "${OVERLAY_STAGE}/." "$WORK_DIR/merged/"

# Logic Validation: Ensure sudo and critical configs survived the fuse
if [ ! -f "$WORK_DIR/merged/usr/bin/sudo" ]; then
    echo "[CRITICAL ERROR] sudo vanished during fusion. Logic collapse detected."
    exit 1
fi

# Create the final fused tarball
tar --numeric-owner -cf "$OUT_TAR" -C "$WORK_DIR/merged" .

echo "--- Fused RootFS Created: $OUT_TAR ---"
