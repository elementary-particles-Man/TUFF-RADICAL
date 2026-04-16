#!/bin/bash
# TUFF-RADICAL: QEMU Live ISO Smoke Test (Serial console output)
set -euo pipefail

export PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
ISO="${DISTRO_DIR}/out/images/live/tuff-live-stable-amd64-minbase.iso"
LOG_DIR="${DISTRO_DIR}/out/tests/boot"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
mkdir -p "$LOG_DIR"

if [ ! -f "$ISO" ]; then
    echo "[ERROR] ISO not found: $ISO"
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
    local log="${LOG_DIR}/$(basename "$ISO").${mode}.serial.log"
    local vars_tmp=""
    local qemu_status=0

    rm -f "${log}"
    echo "--- Testing Live ISO Boot: ${mode} ---"
    
    # ISO 起動の QEMU 引数
    # -cdrom または -drive file=$ISO,media=cdrom
    # シリアルコンソールに出力を促すための boot パラメータ追加が必要な場合がある (GRUB/ISOLINUX 側で設定済みを前提)
    local -a qemu_args=(
        "-boot" "d"
        "-cdrom" "${ISO}"
        "-serial" "file:${log}"
        "-display" "none"
        "-monitor" "none"
        "-no-reboot"
        "-m" "2048"
        "-device" "virtio-net-pci,netdev=net0"
        "-netdev" "user,id=net0"
    )
    
    if [ "${mode}" = "uefi" ]; then
        if [ -n "${OVMF_VARS}" ]; then
            # テンポラリのVARSファイルを作成して書き込み可能にする
            vars_tmp="$(mktemp)"
            cp "${OVMF_VARS}" "${vars_tmp}"
            chmod 644 "${vars_tmp}"
            qemu_args+=("-drive" "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}")
            qemu_args+=("-drive" "if=pflash,format=raw,file=${vars_tmp}")
        else
            qemu_args+=("-bios" "${OVMF_CODE}")
        fi
    fi

    # 120秒でタイムアウト (ISO は RAW よりも起動が遅いため)
    if ! timeout 180 "${QEMU_BIN}" "${qemu_args[@]}" >/dev/null 2>&1; then
        qemu_status=$?
    fi
    
    if [ -n "${vars_tmp}" ]; then
        rm -f "${vars_tmp}"
    fi

    if [ "${qemu_status}" -ne 0 ] && [ "${qemu_status}" -ne 124 ]; then
        echo "[FAILED] ${mode} live boot crashed before timeout (QEMU exit: ${qemu_status})"
        return 1
    fi

    if grep -Eai "login:|Welcome to TUFF-RADICAL|Debian GNU/Linux|live-boot" "${log}" >/dev/null; then
        echo "[OK] ${mode} live boot successful (found expected string in serial log)"
        return 0
    fi

    echo "[FAILED] ${mode} live boot failed or timed out (check ${log})"
    echo "Last lines of log:"
    tail -n 20 "${log}" 2>/dev/null || true
    return 1
}

run_test "bios"
run_test "uefi"
