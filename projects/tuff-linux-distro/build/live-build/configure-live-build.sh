#!/bin/bash
# TUFF-RADICAL: Configure Live-Build Workspace
set -e

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}
mkdir -p "$LB_WORK_DIR"
cd "$LB_WORK_DIR"

echo "--- TUFF Linux Distro: Configuring Live-Build ---"

# 1. ワークスペースの初期化
lb config \
    --distribution trixie \
    --debian-installer live \
    --archive-areas "main" \
    --binary-images iso-hybrid \
    --bootloader grub-efi \
    --iso-application "TUFF-RADICAL" \
    --iso-publisher "TUFF-RADICAL" \
    --iso-volume "TUFF-RADICAL-LIVE" \
    --memtest none \
    --linux-packages linux-image-amd64 \
    --apt-recommends false \
    --firmware-binary true \
    --firmware-chroot true

# 2. パッケージリストの生成
# tuff-base と tuff-recovery のマニフェストから生成
cat "${DISTRO_DIR}/packages/tuff-base/manifest.txt" | grep -v '^#' | grep -v '^$' > config/package-lists/tuff-live.list.chroot
cat "${DISTRO_DIR}/packages/tuff-recovery/manifest.txt" | grep -v '^#' | grep -v '^$' >> config/package-lists/tuff-live.list.chroot

# 3. Overlay の同期
mkdir -p config/includes.chroot
cp -r "${DISTRO_DIR}/overlay/"* config/includes.chroot/

# 4. GRUB シリアルコンソール設定の追加 (live 用)
# config/hooks/0100-grub-serial.chroot などのフックが必要になるかもしれないが、
# まずは基本構成を完了させる。

echo "--- Live-Build Configured in: $LB_WORK_DIR ---"
echo "Next: sudo ./build/live-build/build-live-image.sh"
