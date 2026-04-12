#!/bin/bash
set -e

# PATH を build/release/test 等で必要なものに固定
export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

echo "--- TUFF Linux Distro: Build Host Preflight Check ---"

# 1. 依存コマンドの確認
DEPS=("mmdebstrap" "parted" "losetup" "grub-install" "mkfs.ext4" "xorriso" "lb" "timeout")
MISSING=0

for cmd in "${DEPS[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "[ERROR] Missing command: $cmd"
        MISSING=1
    else
        echo "[OK] Found: $cmd ($(command -v $cmd))"
    fi
done

if [ $MISSING -eq 1 ]; then
    echo "Please install missing dependencies (apt-get install mmdebstrap parted xorriso live-build grub-common grub-pc-bin grub-efi-amd64-bin ...)"
    exit 1
fi

# 2. root 権限の確認 (loop device 操作や chroot に必要)
if [ "$EUID" -ne 0 ]; then
    echo "[WARNING] Not running as root. Root privileges will be required for VM image creation and mmdebstrap (unshare/fake-root mode might work for tarball creation)."
fi

# 3. live-build workspace の確認
LB_WORK_DIR=${LB_WORK_DIR:-/var/tmp/tuff-live-build-work}
if [ ! -d "$LB_WORK_DIR" ]; then
    echo "[INFO] Live-build workspace directory $LB_WORK_DIR does not exist. It will be created during configure phase."
fi

echo "--- Preflight check passed ---"
