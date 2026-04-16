#!/bin/bash
# TUFF-RADICAL: Configure Live-Build Workspace
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_RENDER="${DISTRO_DIR}/build/common/render-package-list.sh"
PRESEED_FILE="${DISTRO_DIR}/build/live-build/tuff-installer.preseed"
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}

if [ ! -x "${PACKAGE_RENDER}" ]; then
    echo "[ERROR] Missing package renderer: ${PACKAGE_RENDER}"
    exit 1
fi

if [ ! -f "${PRESEED_FILE}" ]; then
    echo "[ERROR] Missing preseed file: ${PRESEED_FILE}"
    exit 1
fi

mkdir -p "$LB_WORK_DIR"
cd "$LB_WORK_DIR"

echo "--- TUFF Linux Distro: Configuring Live-Build ---"

# 古い config を残したまま lb config を重ねると壊れた設定が残る。
rm -rf config auto local
lb clean --purge >/dev/null 2>&1 || true

# 1. ワークスペースの初期化
lb config \
    --distribution trixie \
    --debian-installer live \
    --debian-installer-gui false \
    --archive-areas "main contrib non-free non-free-firmware" \
    --binary-images iso-hybrid \
    --bootloaders "syslinux,grub-efi" \
    --loadlin false \
    --iso-application "TUFF-RADICAL" \
    --iso-publisher "TUFF-RADICAL" \
    --iso-volume "TUFF-RADICAL-LIVE" \
    --memtest none \
    --linux-packages "linux-image" \
    --linux-flavours "amd64" \
    --apt-recommends true \
    --firmware-binary true \
    --firmware-chroot true \
    --bootappend-live "boot=live components locales=ja_JP.UTF-8 keyboard-layouts=jp timezone=Asia/Tokyo console=tty0 console=ttyS0,115200n8" \
    --bootappend-install "auto=true priority=critical locale=ja_JP.UTF-8 console=tty0 console=ttyS0,115200n8"

# 2. パッケージリストの生成
mkdir -p config/package-lists
"${PACKAGE_RENDER}" tuff-base tuff-recovery > config/package-lists/tuff-live.list.chroot

# 3. Overlay の同期
mkdir -p config/includes.chroot
cp -a "${DISTRO_DIR}/overlay/." config/includes.chroot/

# 4. Preseed の適用
mkdir -p config/preseed
install -m 0644 "${PRESEED_FILE}" config/preseed/tuff.preseed.binary
install -m 0644 "${PRESEED_FILE}" config/preseed/tuff.preseed.chroot

# 4. GRUB シリアルコンソール設定の追加 (live 用)
# config/hooks/0100-grub-serial.chroot などのフックが必要になるかもしれないが、
# まずは基本構成を完了させる。

echo "--- Live-Build Configured in: $LB_WORK_DIR ---"
echo "Next: sudo ./build/live-build/build-live-image.sh"
