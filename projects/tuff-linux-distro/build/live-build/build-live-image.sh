#!/bin/bash
# TUFF-RADICAL: Build Live ISO Image
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}
OUT_ISO_DIR="${DISTRO_DIR}/out/images/live"
OUT_ISO="${OUT_ISO_DIR}/tuff-live-stable-amd64-minbase.iso"
mkdir -p "$OUT_ISO_DIR"

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] This script must be run as root (sudo)."
    exit 1
fi

if [ ! -d "${LB_WORK_DIR}/config" ]; then
    echo "[ERROR] Live-build workspace is not configured: ${LB_WORK_DIR}"
    exit 1
fi

cd "$LB_WORK_DIR"

echo "--- TUFF Linux Distro: Building Live ISO ---"
echo "Workspace: $LB_WORK_DIR"

rm -f ./*.iso
lb clean --binary >/dev/null 2>&1 || true

# ビルド実行
lb build

ISO_FILE="$(find . -maxdepth 1 -type f -name '*.iso' -printf '%f\n' | head -n 1)"
if [ -n "${ISO_FILE}" ] && [ -f "${ISO_FILE}" ]; then
    mv -f "${ISO_FILE}" "${OUT_ISO}"
    echo "--- Live ISO Created: ${OUT_ISO} ---"
else
    echo "[ERROR] ISO generation failed (no .iso file found in $LB_WORK_DIR)"
    exit 1
fi
