#!/bin/bash
# TUFF-RADICAL: QEMU Boot Smoke Test (Serial console output)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
IMG="${DISTRO_DIR}/out/images/vm/tuff-vm-stable-amd64-minbase.raw"
LOG_DIR="${DISTRO_DIR}/out/tests/boot"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
mkdir -p "$LOG_DIR"

if [ ! -f "$IMG" ]; then
    echo "[ERROR] VM image not found: $IMG"
    exit 1
fi

if ! command -v "${QEMU_BIN}" >/dev/null 2>&1; then
    echo "[ERROR] Missing QEMU binary: ${QEMU_BIN}"
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
    local mode="$1"
    local log="${LOG_DIR}/$(basename "$IMG").${mode}.serial.log"
    local vars_tmp=""
    local qemu_status=0

    rm -f "$log"
    echo "--- Testing Boot: ${mode} ---"

    local -a qemu_args=(
        "-drive" "file=${IMG},format=raw,if=virtio"
        "-serial" "file:${log}"
        "-display" "none"
        "-monitor" "none"
        "-no-reboot"
        "-m" "1024"
        "-device" "virtio-net-pci,netdev=net0"
        "-netdev" "user,id=net0"
    )

    if [ "${mode}" = "uefi" ]; then
        if [ -n "$OVMF_VARS" ]; then
            vars_tmp="$(mktemp)"
            cp "${OVMF_VARS}" "${vars_tmp}"
            chmod 644 "${vars_tmp}"
            qemu_args+=("-drive" "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}")
            qemu_args+=("-drive" "if=pflash,format=raw,file=${vars_tmp}")
        else
            qemu_args+=("-bios" "${OVMF_CODE}")
        fi
    fi

    if ! timeout 60 "${QEMU_BIN}" "${qemu_args[@]}" >/dev/null 2>&1; then
        qemu_status=$?
    fi

    if [ -n "${vars_tmp}" ]; then
        rm -f "${vars_tmp}"
    fi

    if [ "${qemu_status}" -ne 0 ] && [ "${qemu_status}" -ne 124 ]; then
        echo "[FAILED] ${mode} boot crashed before timeout (QEMU exit: ${qemu_status})"
        return 1
    fi

    if grep -Eai "login:|Welcome to TUFF-RADICAL" "${log}" >/dev/null; then
        echo "[OK] ${mode} boot successful (found expected string in serial log)"
        return 0
    fi

    echo "[FAILED] ${mode} boot failed or timed out (check ${log})"
    tail -n 20 "${log}" 2>/dev/null || true
    return 1
}

run_test "bios"
run_test "uefi"
