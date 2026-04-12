#!/bin/bash
# TUFF-RADICAL: Surgical Disk Installer v3 (Pure Logos)
set -e

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

TARGET_DEV=$1
ROOTFS_TAR=$2
OVERLAY_DIR="$(cd "$(dirname "$0")/../.." && pwd)/overlay"

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] Root privileges required."
    exit 1
fi

if [ -z "$TARGET_DEV" ] || [ -z "$ROOTFS_TAR" ]; then
    echo "Usage: sudo $0 <target_device> <rootfs_tarball>"
    exit 1
fi

echo "--- Installing TUFF-RADICAL to $TARGET_DEV ---"

# 1. 物理的な設置 (Partitioning)
parted -s "$TARGET_DEV" mklabel gpt \
    mkpart bios_boot 1MiB 2MiB \
    set 1 bios_grub on \
    mkpart ESP fat32 2MiB 128MiB \
    set 2 esp on \
    mkpart root ext4 128MiB 100%

partprobe "$TARGET_DEV"
udevadm settle

# 2. フォーマット
if [[ "$TARGET_DEV" == /dev/nvme* ]] || [[ "$TARGET_DEV" == /dev/mmcblk* ]]; then
    P2="${TARGET_DEV}p2"; P3="${TARGET_DEV}p3"
else
    P2="${TARGET_DEV}2"; P3="${TARGET_DEV}3"
fi

mkfs.vfat -F 32 -n TUFF_ESP "$P2"
mkfs.ext4 -F -L TUFF_ROOT "$P3"

# 3. マウントと配置 (Mount & Extract)
MNT_DIR="/tmp/tuff-install-mnt"
mkdir -p "$MNT_DIR"
mount "$P3" "$MNT_DIR"
mkdir -p "$MNT_DIR/boot/efi"
mount "$P2" "$MNT_DIR/boot/efi"
trap "umount -l $MNT_DIR/boot/efi $MNT_DIR || true" EXIT

tar -xf "$ROOTFS_TAR" -C "$MNT_DIR"

# Overlay の適用 (Sovereign Configs)
if [ -d "$OVERLAY_DIR" ]; then
    cp -rv "$OVERLAY_DIR/"* "$MNT_DIR/"
fi

# 4. fstab の最小設定
UUID_ROOT=$(blkid -s UUID -o value "$P3")
UUID_ESP=$(blkid -s UUID -o value "$P2")
cat <<EOF > "$MNT_DIR/etc/fstab"
UUID=$UUID_ROOT  /      ext4  errors=remount-ro,noatime  0  1
UUID=$UUID_ESP   /boot/efi  vfat  umask=0077  0  1
tmpfs            /tmp   tmpfs defaults,nosuid,nodev  0  0
EOF

# 5. ブートローダの設置 (Bootloader Installation)
mount --bind /dev "$MNT_DIR/dev"
mount --bind /proc "$MNT_DIR/proc"
mount --bind /sys "$MNT_DIR/sys"
trap "umount -l $MNT_DIR/dev $MNT_DIR/proc $MNT_DIR/sys $MNT_DIR/boot/efi $MNT_DIR || true" EXIT

grub-install --target=i386-pc --boot-directory="$MNT_DIR/boot" "$TARGET_DEV"
grub-install --target=x86_64-efi --efi-directory="$MNT_DIR/boot/efi" --boot-directory="$MNT_DIR/boot" --removable --no-nvram
chroot "$MNT_DIR" update-grub

echo "--- TUFF-RADICAL: Installation Finished ---"
