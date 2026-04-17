#!/bin/bash
# TUFF-RADICAL: Surgical Disk Installer v4 (Hardened Logos)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

TARGET_DEV="${1:-}"
ROOTFS_TAR="${2:-}"
TUFF_USER="${TUFF_USER:-flux}"
OVERLAY_DIR="$(cd "$(dirname "$0")/../.." && pwd)/overlay"
MNT_DIR="/tmp/tuff-install-mnt"

cleanup() {
    echo "[INFO] Cleaning up mount points..."
    umount -l "${MNT_DIR}/sys/firmware/efi/efivars" 2>/dev/null || true
    umount -l "${MNT_DIR}/dev" 2>/dev/null || true
    umount -l "${MNT_DIR}/proc" 2>/dev/null || true
    umount -l "${MNT_DIR}/sys" 2>/dev/null || true
    umount -l "${MNT_DIR}/boot/efi" 2>/dev/null || true
    umount -l "${MNT_DIR}" 2>/dev/null || true
}

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] Root privileges required."
    exit 1
fi

if [ -z "$TARGET_DEV" ] || [ -z "$ROOTFS_TAR" ]; then
    echo "Usage: sudo $0 <target_device> <rootfs_tarball>"
    exit 1
fi

if [ ! -b "${TARGET_DEV}" ]; then
    echo "[ERROR] Target device is not a block device: ${TARGET_DEV}"
    exit 1
fi

if [ ! -f "${ROOTFS_TAR}" ]; then
    echo "[ERROR] Rootfs tarball not found: ${ROOTFS_TAR}"
    exit 1
fi

echo "--- Installing TUFF-RADICAL to $TARGET_DEV ---"
trap cleanup EXIT

# 1. 物理的な設置 (Partitioning with optimal alignment)
parted -s "$TARGET_DEV" --align optimal mklabel gpt \
    mkpart bios_boot 1MiB 2MiB \
    set 1 bios_grub on \
    mkpart ESP fat32 2MiB 512MiB \
    set 2 esp on \
    mkpart root ext4 512MiB 100%

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
mkdir -p "$MNT_DIR"
mount "$P3" "$MNT_DIR"
mkdir -p "$MNT_DIR/boot/efi"
mount "$P2" "$MNT_DIR/boot/efi"

# Use numeric owner to ensure identity consistency
tar --numeric-owner -xf "$ROOTFS_TAR" -C "$MNT_DIR"

# 4. Logical Identity (User & Permission)
echo "[INFO] Establishing User Identity & Sudoers..."
# Verify sudo package exists
if ! chroot "$MNT_DIR" which sudo >/dev/null 2>&1; then
    echo "[CRITICAL ERROR] sudo package is MISSING in the rootfs. Aborting."
    exit 1
fi

chroot "$MNT_DIR" groupadd -f sudo 2>/dev/null || true
chroot "$MNT_DIR" id -u "$TUFF_USER" >/dev/null 2>&1 || chroot "$MNT_DIR" useradd -m -s /bin/bash -G sudo "$TUFF_USER"
chroot "$MNT_DIR" usermod -aG audio,video,netdev,plugdev,bluetooth,lpadmin "$TUFF_USER" 2>/dev/null || true

# Overlay の適用 (T-RAD Configs)
if [ -d "$OVERLAY_DIR" ]; then
    cp -a "${OVERLAY_DIR}/." "$MNT_DIR/"
    if [ -f "$MNT_DIR/etc/sudoers.d/tuff" ]; then
        chown root:root "$MNT_DIR/etc/sudoers.d/tuff"
        chmod 440 "$MNT_DIR/etc/sudoers.d/tuff"
    fi
fi

# 5. fstab の最小設定 (Noatime for SSD performance)
UUID_ROOT=$(blkid -s UUID -o value "$P3")
UUID_ESP=$(blkid -s UUID -o value "$P2")
cat <<FSTAB_EOF > "$MNT_DIR/etc/fstab"
UUID=$UUID_ROOT  /      ext4  errors=remount-ro,noatime  0  1
UUID=$UUID_ESP   /boot/efi  vfat  umask=0077  0  1
tmpfs            /tmp   tmpfs defaults,nosuid,nodev  0  0
FSTAB_EOF

# 6. ブートローダの設置 (Bootloader Installation)
mount --bind /dev "$MNT_DIR/dev"
mount --bind /proc "$MNT_DIR/proc"
mount --bind /sys "$MNT_DIR/sys"
mount -t efivarfs efivarfs "$MNT_DIR/sys/firmware/efi/efivars" 2>/dev/null || true

export DEBIAN_FRONTEND=noninteractive
grub-install --target=i386-pc --boot-directory="$MNT_DIR/boot" "$TARGET_DEV"
grub-install --target=x86_64-efi --efi-directory="$MNT_DIR/boot/efi" --boot-directory="$MNT_DIR/boot" --removable --no-nvram
chroot "$MNT_DIR" update-grub

echo "--- TUFF-RADICAL: Installation Finished ---"
