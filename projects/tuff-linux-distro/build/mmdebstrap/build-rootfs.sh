#!/bin/bash
set -e

# PATH を build/release/test 等で必要なものに固定
export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_LIST="${DISTRO_DIR}/packages/tuff-base/tuff-base.txt"
OUT_TAR="${DISTRO_DIR}/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar"
TUFF_USER="${TUFF_USER:-$(id -un)}"

echo "--- TUFF Linux Distro: Building RootFS Tarball (Target User: ${TUFF_USER}) ---"

# パッケージリストから改行を除いてカンマ区切りにする
PACKAGES=$(grep -v '^#' "$PACKAGE_LIST" | grep -v '^$' | xargs | tr ' ' ',')

# mmdebstrap 実行 (rootless/unshare mode を優先)
mmdebstrap \
    --variant=minbase \
    --components="main,contrib,non-free,non-free-firmware" \
    --include="$PACKAGES" \
    --customize-hook="chroot \"\$1\" groupadd -f sudo" \
    --customize-hook="chroot \"\$1\" useradd -m -s /bin/bash -G sudo \"${TUFF_USER}\" 2>/dev/null || true" \
    --customize-hook="chroot \"\$1\" usermod -aG audio,video,netdev,plugdev,bluetooth,lpadmin \"${TUFF_USER}\" 2>/dev/null || true" \
    --customize-hook="cp -r \"${DISTRO_DIR}/overlay/\"* \"\$1\"/" \
    --customize-hook="chroot \"\$1\" chown root:root /etc/sudoers.d/tuff 2>/dev/null || true" \
    --customize-hook="chroot \"\$1\" chmod 440 /etc/sudoers.d/tuff 2>/dev/null || true" \
    trixie \
    "$OUT_TAR" \
    http://deb.debian.org/debian/

echo "--- RootFS Tarball Created: $OUT_TAR ---"
