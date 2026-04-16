#!/bin/bash
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
DISTRO_DIR="${REPO_DIR}/projects/tuff-linux-distro"
ISO_PATH="${ISO_PATH:-${DISTRO_DIR}/out/images/live/tuff-live-stable-amd64-minbase.iso}"

usage() {
    cat <<'EOF'
usage: tuff-build-usb.sh [/dev/sdX]

Build the TUFF live ISO if needed, then write it to a removable USB flash drive.
When no target device is given, the script auto-selects the single removable USB disk.
EOF
}

detect_target_dev() {
    local -a candidates=()

    mapfile -t candidates < <(lsblk -dnpo NAME,TRAN,RM,TYPE | awk '$2 == "usb" && $3 == "1" && $4 == "disk" { print $1 }')

    if [ "${#candidates[@]}" -eq 0 ]; then
        echo "[ERROR] No removable USB disk was detected." >&2
        exit 1
    fi

    if [ "${#candidates[@]}" -ne 1 ]; then
        echo "[ERROR] Multiple removable USB disks were detected. Pass the target device explicitly." >&2
        printf ' - %s\n' "${candidates[@]}" >&2
        exit 1
    fi

    printf '%s\n' "${candidates[0]}"
}

TARGET_DEV="${1:-}"
if [ "${TARGET_DEV}" = "-h" ] || [ "${TARGET_DEV}" = "--help" ]; then
    usage
    exit 0
fi

if [ -z "${TARGET_DEV}" ]; then
    TARGET_DEV="$(detect_target_dev)"
fi

if [ ! -b "${TARGET_DEV}" ]; then
    echo "[ERROR] Target device does not exist or is not a block device: ${TARGET_DEV}" >&2
    exit 1
fi

if [ ! -f "${ISO_PATH}" ]; then
    echo "--- Live ISO not found. Building a fresh image first. ---"
    "${DISTRO_DIR}/build/live-build/configure-live-build.sh"
    sudo "${DISTRO_DIR}/build/live-build/build-live-image.sh"
fi

if [ ! -f "${ISO_PATH}" ]; then
    echo "[ERROR] ISO not found after build: ${ISO_PATH}" >&2
    exit 1
fi

ISO_SIZE="$(stat -c %s "${ISO_PATH}")"
DEV_SIZE="$(sudo blockdev --getsize64 "${TARGET_DEV}")"

if [ "${ISO_SIZE}" -gt "${DEV_SIZE}" ]; then
    echo "[ERROR] ISO is larger than the target device." >&2
    echo "ISO bytes: ${ISO_SIZE}" >&2
    echo "DEV bytes: ${DEV_SIZE}" >&2
    exit 1
fi

echo "--- Target USB device: ${TARGET_DEV} ---"
lsblk -o NAME,MODEL,SIZE,TRAN,RM,MOUNTPOINTS "${TARGET_DEV}"

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_dir="${DISTRO_DIR}/out/usb-backups/${timestamp}-$(basename "${TARGET_DEV}")-backup"
mkdir -p "${backup_dir}"

lsblk -o NAME,MODEL,SIZE,FSTYPE,UUID,MOUNTPOINTS "${TARGET_DEV}" > "${backup_dir}/lsblk.txt"
blkid "${TARGET_DEV}"* > "${backup_dir}/blkid.txt" 2>/dev/null || true
sudo sfdisk --dump "${TARGET_DEV}" > "${backup_dir}/partition-table.sfdisk" 2>/dev/null || true

while read -r part part_type; do
    if [ "${part_type}" = "part" ] && findmnt -rn -S "${part}" >/dev/null 2>&1; then
        sudo umount "${part}"
    fi
done < <(lsblk -lnpo NAME,TYPE "${TARGET_DEV}")

echo "--- Writing ${ISO_PATH} to ${TARGET_DEV} ---"
sudo dd if="${ISO_PATH}" of="${TARGET_DEV}" bs=16M conv=fsync,notrunc oflag=direct status=progress
sync
sudo partprobe "${TARGET_DEV}" || true
sudo udevadm settle || true

echo "--- Verifying written bytes ---"
sudo cmp -n "${ISO_SIZE}" "${ISO_PATH}" "${TARGET_DEV}" >/dev/null

lsblk -o NAME,MODEL,SIZE,FSTYPE,MOUNTPOINTS "${TARGET_DEV}" > "${backup_dir}/post-write-lsblk.txt"
cat "${backup_dir}/post-write-lsblk.txt"

echo "--- USB write complete and verified. ---"
