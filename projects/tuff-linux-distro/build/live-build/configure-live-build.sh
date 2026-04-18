#!/bin/bash
# TUFF-RADICAL: Configure Live-Build Workspace v4 (Hardened Sync)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_RENDER="${DISTRO_DIR}/build/common/render-package-list.sh"
PREPARE_OVERLAY="${DISTRO_DIR}/build/common/prepare-overlay.sh"
PRESEED_FILE="${DISTRO_DIR}/build/live-build/tuff-installer.preseed"
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}

if [ ! -x "${PACKAGE_RENDER}" ]; then
    echo "[ERROR] Missing package renderer: ${PACKAGE_RENDER}"
    exit 1
fi

mkdir -p "$LB_WORK_DIR"
cd "$LB_WORK_DIR"

echo "--- TUFF Linux Distro: Configuring Live-Build ---"

# Clean start to prevent "Zombie Configs"
rm -rf config auto local
lb clean --purge >/dev/null 2>&1 || true

# 1. Base Configuration with Robust USB Params
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
    --bootappend-live "boot=live components locales=ja_JP.UTF-8 keyboard-layouts=jp timezone=Asia/Tokyo console=tty0 pcie_aspm=off pci=noaer random.trust_cpu=on amd_pstate=active" \
    --bootappend-install "auto=true priority=critical locale=ja_JP.UTF-8 console=tty0"

# 2. Package List Generation using the Unified Renderer
mkdir -p config/package-lists
"${PACKAGE_RENDER}" tuff-base > config/package-lists/tuff-live.list.chroot

# 3. Overlay Sync (Atomic)
OVERLAY_STAGE="$("${PREPARE_OVERLAY}")"
mkdir -p config/includes.chroot
cp -a "${OVERLAY_STAGE}/." config/includes.chroot/

# DNS Fix for firmware-chroot
mkdir -p config/includes.chroot/etc
cp /etc/resolv.conf config/includes.chroot/etc/resolv.conf

# 4. Preseed Application
if [ -f "${PRESEED_FILE}" ]; then
    mkdir -p config/preseed
    install -m 0644 "${PRESEED_FILE}" config/preseed/tuff.preseed.binary
    install -m 0644 "${PRESEED_FILE}" config/preseed/tuff.preseed.chroot
fi

echo "--- Live-Build Configured successfully ---"
