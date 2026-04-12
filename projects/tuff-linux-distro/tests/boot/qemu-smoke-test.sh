#!/bin/bash
# TUFF-RADICAL: QEMU Boot Smoke Test (Serial console output)
set -e

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
IMG="${DISTRO_DIR}/out/images/vm/tuff-vm-stable-amd64-minbase.raw"
LOG_DIR="${DISTRO_DIR}/out/tests/boot"
mkdir -p "$LOG_DIR"

if [ ! -f "$IMG" ]; then
    echo "[ERROR] VM image not found: $IMG"
    exit 1
fi

# OVMF パスの検出
OVMF_CODE="/usr/share/OVMF/OVMF_CODE_4M.fd"
OVMF_VARS="/usr/share/OVMF/OVMF_VARS_4M.fd"

if [ ! -f "$OVMF_CODE" ]; then
    # Fallback to standard path
    OVMF_CODE="/usr/share/ovmf/OVMF.fd"
    OVMF_VARS=""
fi

run_test() {
    MODE=$1 # "bios" or "uefi"
    LOG="${LOG_DIR}/$(basename "$IMG").${MODE}.serial.log"
    echo "--- Testing Boot: $MODE ---"
    
    QEMU_ARGS=("-drive" "file=$IMG,format=raw,if=virtio" "-serial" "file:$LOG" "-display" "none" "-m" "1024" "-device" "virtio-net-pci,netdev=net0" "-netdev" "user,id=net0")
    
    if [ "$MODE" == "uefi" ]; then
        if [ -n "$OVMF_VARS" ]; then
            QEMU_ARGS+=("-drive" "if=pflash,format=raw,readonly=on,file=$OVMF_CODE")
            QEMU_ARGS+=("-drive" "if=pflash,format=raw,file=$OVMF_VARS")
        else
            QEMU_ARGS+=("-bios" "$OVMF_CODE")
        fi
    fi

    # 60秒でタイムアウト (GRUBメニューや初期化待機)
    timeout 60 qemu-system-x86_64 "${QEMU_ARGS[@]}" || true
    
    if grep -Ei "login:|Welcome to TUFF-RADICAL" "$LOG" > /dev/null; then
        echo "[OK] $MODE boot successful (Found login/welcome in serial log)"
    else
        echo "[FAILED] $MODE boot failed or timed out (Check $LOG)"
    fi
}

run_test "bios"
run_test "uefi"
