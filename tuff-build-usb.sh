#!/bin/bash
# TUFF-RADICAL: Surgical USB Media Builder v4 (Safe & Hardened)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
DISTRO_DIR="${REPO_DIR}/projects/tuff-linux-distro"
# Adjust path to match the actual output of live-build
ISO_PATH="${REPO_DIR}/projects/tuff-linux-distro/out/images/live/tuff-live-stable-amd64-minbase.iso"

usage() {
    cat <<'EOF'
usage: tuff-build-usb.sh [/dev/sdX]

Build the TUFF live ISO and write it safely to a removable USB flash drive.
Fails if target is not a removable USB device to prevent accidental data loss.
EOF
}

detect_target_dev() {
    local -a candidates=()
    # Be more strict: only USB, removable, and DISK type.
    mapfile -t candidates < <(lsblk -dnpo NAME,TRAN,RM,TYPE | awk '$2 == "usb" && $3 == "1" && $4 == "disk" { print $1 }')

    if [ "${#candidates[@]}" -eq 0 ]; then
        echo "[ERROR] No removable USB disk was detected." >&2
        exit 1
    fi

    if [ "${#candidates[@]}" -ne 1 ]; then
        echo "[ERROR] Ambiguous targets found. Pass the device explicitly (e.g., /dev/sdX)." >&2
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

# Safety Check: Must be a block device and removable USB
if ! lsblk -dnpo NAME,TRAN,RM | grep -q "^${TARGET_DEV}\s\+usb\s\+1$"; then
    echo "[CRITICAL ERROR] ${TARGET_DEV} is NOT a removable USB device. Refusing to destroy potentially internal data." >&2
    exit 1
fi

# Ensure ISO exists
if [ ! -f "${ISO_PATH}" ]; then
    echo "--- Live ISO not found at ${ISO_PATH}. Building fresh image... ---"
    # Execute full build pipeline
    "${DISTRO_DIR}/build/live-build/configure-live-build.sh"
    sudo "${DISTRO_DIR}/build/live-build/build-live-image.sh"
fi

if [ ! -f "${ISO_PATH}" ]; then
    echo "[ERROR] ISO build failed or not found at: ${ISO_PATH}" >&2
    exit 1
fi

# Check size
ISO_SIZE="$(stat -c %s "${ISO_PATH}")"
DEV_SIZE="$(sudo blockdev --getsize64 "${TARGET_DEV}")"

if [ "${ISO_SIZE}" -gt "${DEV_SIZE}" ]; then
    echo "[ERROR] ISO (${ISO_SIZE} bytes) is larger than target ${TARGET_DEV} (${DEV_SIZE} bytes)." >&2
    exit 1
fi

echo "--- TARGET: ${TARGET_DEV} ---"
lsblk -o NAME,MODEL,SIZE,TRAN,RM,MOUNTPOINTS "${TARGET_DEV}"

# Unmount all partitions forcefully
echo "--- Unmounting partitions on ${TARGET_DEV} ---"
for part in "${TARGET_DEV}"*; do
    if [ -b "$part" ]; then
        sudo umount -l "$part" 2>/dev/null || true
    fi
done

# Perform the write with verification
echo "--- Writing ISO to ${TARGET_DEV} (with fsync & direct I/O) ---"
sudo dd if="${ISO_PATH}" of="${TARGET_DEV}" bs=16M conv=fsync,notrunc oflag=direct status=progress

echo "--- Flushing buffers (Final Sync) ---"
sync

# Final Verification
echo "--- Verifying integrity... ---"
sudo cmp -n "${ISO_SIZE}" "${ISO_PATH}" "${TARGET_DEV}"
if [ $? -eq 0 ]; then
    echo "--- [SUCCESS] USB write complete and verified. ---"
else
    echo "--- [FATAL ERROR] Verification failed. Data on USB is corrupted. ---"
    exit 1
fi

sudo partprobe "${TARGET_DEV}" 2>/dev/null || true
sudo udevadm settle 2>/dev/null || true
