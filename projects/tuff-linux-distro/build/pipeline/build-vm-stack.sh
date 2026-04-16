#!/bin/bash
set -e

# PATH を build/release/test 等で必要なものに固定
export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
ROOTFS_TAR="${DISTRO_DIR}/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar"
OUT_IMG="${DISTRO_DIR}/out/images/vm/tuff-vm-stable-amd64-minbase.raw"
MNT_DIR="${DISTRO_DIR}/out/mnt_vm_root"

if [ "$EUID" -ne 0 ]; then
    echo "[ERROR] This script must be run as root (sudo)."
    exit 1
fi

if [ ! -f "$ROOTFS_TAR" ]; then
    echo "[ERROR] RootFS tarball not found: $ROOTFS_TAR"
    exit 1
fi

mkdir -p "$(dirname "$OUT_IMG")"
mkdir -p "$MNT_DIR"

echo "--- TUFF Linux Distro: Creating VM Image ---"

# 1. 8GB の空のイメージを作成
dd if=/dev/zero of="$OUT_IMG" bs=1M count=8192

# 2. パーティション作成 (BIOS+UEFI 両対応: GPT + BIOS Boot Partition + ESP + Root)
parted -s "$OUT_IMG" mklabel gpt
parted -s "$OUT_IMG" mkpart bios_boot 1MiB 2MiB
parted -s "$OUT_IMG" set 1 bios_grub on
parted -s "$OUT_IMG" mkpart ESP fat32 2MiB 128MiB
parted -s "$OUT_IMG" set 2 esp on
parted -s "$OUT_IMG" mkpart root ext4 128MiB 100%

# 3. loop device へのアタッチ
LOOP_DEV=$(losetup -fP --show "$OUT_IMG")
trap "losetup -d $LOOP_DEV" EXIT

# 4. フォーマット
mkfs.vfat -F 32 "${LOOP_DEV}p2"
mkfs.ext4 -F "${LOOP_DEV}p3"

# 5. マウント
mount "${LOOP_DEV}p3" "$MNT_DIR"
trap "umount -R $MNT_DIR && losetup -d $LOOP_DEV" EXIT

mkdir -p "$MNT_DIR/boot/efi"
mount "${LOOP_DEV}p2" "$MNT_DIR/boot/efi"

# 6. rootfs 展開
tar -xf "$ROOTFS_TAR" -C "$MNT_DIR"

# 7. カーネル/Initrd のシンボリックリンク (必要なら)
# mmdebstrap で linux-image を入れた場合、/vmlinuz 等ができているはず

# 8. GRUB インストール (BIOS)
grub-install --target=i386-pc --boot-directory="$MNT_DIR/boot" "$LOOP_DEV"

# 9. GRUB インストール (UEFI)
grub-install --target=x86_64-efi --efi-directory="$MNT_DIR/boot/efi" --boot-directory="$MNT_DIR/boot" --removable --no-nvram

# 10. GRUB 設定生成
# chroot 内で update-grub するための準備
mount --bind /dev "$MNT_DIR/dev"
mount --bind /proc "$MNT_DIR/proc"
mount --bind /sys "$MNT_DIR/sys"

# シリアルコンソールの有効化
cat <<EOF > "$MNT_DIR/etc/default/grub"
GRUB_DEFAULT=0
GRUB_TIMEOUT=1
GRUB_DISTRIBUTOR="TUFF-RADICAL"
GRUB_CMDLINE_LINUX_DEFAULT="console=ttyS0,115200n8 quiet"
GRUB_CMDLINE_LINUX=""
GRUB_TERMINAL=serial
GRUB_SERIAL_COMMAND="serial --speed=115200 --unit=0 --word=8 --parity=no --stop=1"
EOF

chroot "$MNT_DIR" update-grub

echo "--- VM Image Created: $OUT_IMG ---"
