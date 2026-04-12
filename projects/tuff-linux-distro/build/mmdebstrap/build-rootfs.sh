#!/bin/bash
set -e

# PATH を build/release/test 等で必要なものに固定
export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_LIST="${DISTRO_DIR}/packages/tuff-base/tuff-base.txt"
OUT_TAR="${DISTRO_DIR}/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar"

echo "--- TUFF Linux Distro: Building RootFS Tarball ---"

# パッケージリストから改行を除いてカンマ区切りにする
PACKAGES=$(grep -v '^#' "$PACKAGE_LIST" | grep -v '^$' | xargs | tr ' ' ',')

# mmdebstrap 実行 (rootless/unshare mode を優先)
mmdebstrap \
    --variant=minbase \
    --include="$PACKAGES" \
    trixie \
    "$OUT_TAR" \
    http://deb.debian.org/debian/

echo "--- RootFS Tarball Created: $OUT_TAR ---"
