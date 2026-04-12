#!/bin/bash
# TUFF-RADICAL: Build Live ISO Image
set -e

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}
OUT_ISO_DIR="${DISTRO_DIR}/out/images/live"
mkdir -p "$OUT_ISO_DIR"

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] This script must be run as root (sudo)."
    exit 1
fi

cd "$LB_WORK_DIR"

echo "--- TUFF Linux Distro: Building Live ISO ---"
echo "Workspace: $LB_WORK_DIR"

# ビルド実行
lb build

# 生成された ISO を移動 (live-image-amd64.hybrid.iso などの名称)
ISO_FILE=$(ls *.iso 2>/dev/null | head -n 1)
if [ -f "$ISO_FILE" ]; then
    mv "$ISO_FILE" "${OUT_ISO_DIR}/tuff-live-stable-amd64-minbase.iso"
    echo "--- Live ISO Created: ${OUT_ISO_DIR}/tuff-live-stable-amd64-minbase.iso ---"
else
    echo "[ERROR] ISO generation failed (no .iso file found in $LB_WORK_DIR)"
    exit 1
fi
