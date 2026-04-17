#!/bin/bash
# TUFF-RADICAL: Surgical RootFS Builder v4 (Unified Logos)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
RENDER_PKG="${DISTRO_DIR}/build/common/render-package-list.sh"
OUT_TAR="${DISTRO_DIR}/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar"
TUFF_USER="${TUFF_USER:-flux}"

echo "--- TUFF Linux Distro: Building RootFS Tarball (Target User: ${TUFF_USER}) ---"

# Use the central renderer to get a clean, comma-separated package list
PACKAGES=$("${RENDER_PKG}" --format comma tuff-base)

# Verify key packages exist in the list
if [[ ! "$PACKAGES" =~ "sudo" ]]; then
    echo "[CRITICAL ERROR] sudo missing from package list. Build aborted."
    exit 1
fi

mkdir -p "$(dirname "$OUT_TAR")"

# mmdebstrap with explicit hardened customize hooks
mmdebstrap \
    --variant=minbase \
    --components="main,contrib,non-free,non-free-firmware" \
    --include="$PACKAGES" \
    --customize-hook="chroot \"\$1\" groupadd -f sudo" \
    --customize-hook="chroot \"\$1\" useradd -m -s /bin/bash -G sudo \"${TUFF_USER}\" 2>/dev/null || true" \
    --customize-hook="chroot \"\$1\" usermod -aG audio,video,netdev,plugdev,bluetooth,lpadmin \"${TUFF_USER}\" 2>/dev/null || true" \
    --customize-hook="cp -a \"${DISTRO_DIR}/overlay/\"* \"\$1\"/" \
    --customize-hook="chroot \"\$1\" chown root:root /etc/sudoers.d/tuff 2>/dev/null || true" \
    --customize-hook="chroot \"\$1\" chmod 440 /etc/sudoers.d/tuff 2>/dev/null || true" \
    trixie \
    "$OUT_TAR" \
    http://deb.debian.org/debian/

echo "--- RootFS Tarball Created: $OUT_TAR ---"
