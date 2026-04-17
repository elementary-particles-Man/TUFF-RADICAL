#!/bin/bash
# TUFF-RADICAL: Build Live ISO & Move to Out v4 (Path Sync)
set -euo pipefail

LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}
DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${DISTRO_DIR}/out/images/live"

if [ ! -d "$LB_WORK_DIR" ]; then
    echo "[ERROR] Live-build work directory not found. Run configure-live-build.sh first."
    exit 1
fi

cd "$LB_WORK_DIR"
echo "--- TUFF Linux Distro: Building Live ISO (Real-time Validation) ---"

# 物理ビルドの実行
sudo lb build

# 成果物の移動 (ここが欠落していた致命的バグ #67)
mkdir -p "$OUT_DIR"
ISO_FILE=$(ls *.iso | head -n 1)

if [ -f "$ISO_FILE" ]; then
    echo "--- Moving Build Artifact: $ISO_FILE to $OUT_DIR ---"
    sudo mv "$ISO_FILE" "${OUT_DIR}/tuff-live-stable-amd64-minbase.iso"
    echo "--- Build Artifact Synchronized. ---"
else
    echo "[ERROR] ISO generation failed inside $LB_WORK_DIR"
    exit 1
fi
