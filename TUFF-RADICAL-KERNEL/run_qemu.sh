#!/bin/bash
set -euo pipefail

# TUFF-RADICAL-KERNEL PoCをビルドしてQEMUでテスト起動するスクリプト

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

# 1. ビルド
echo "Building TUFF-RADICAL-KERNEL..."
cargo build

# 2. ESPディレクトリの準備
ESP_DIR="$DIR/target/esp"
mkdir -p "$ESP_DIR/EFI/BOOT"
cp target/x86_64-unknown-uefi/debug/tuff-radical-kernel.efi "$ESP_DIR/EFI/BOOT/BOOTX64.EFI"

# 3. OVMFファームウェアの設定
OVMF_CODE="/usr/share/OVMF/OVMF_CODE_4M.fd"
OVMF_VARS_TEMPLATE="/usr/share/OVMF/OVMF_VARS_4M.fd"
OVMF_VARS="$DIR/target/OVMF_VARS_4M.fd"

if [[ ! -f "$OVMF_CODE" ]]; then
    echo "OVMF firmware not found. Please install OVMF (e.g., sudo apt install ovmf)"
    exit 1
fi

if [[ ! -f "$OVMF_VARS" ]]; then
    cp "$OVMF_VARS_TEMPLATE" "$OVMF_VARS"
fi

# QEMU KVMの有効化確認
QEMU_ACCEL=()
if [[ -e /dev/kvm && -r /dev/kvm && -w /dev/kvm ]]; then
    QEMU_ACCEL=(-enable-kvm -cpu host)
fi

echo "Starting QEMU..."
exec qemu-system-x86_64 \
    "${QEMU_ACCEL[@]}" \
    -machine q35 \
    -m 512M \
    -vga std \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file="$OVMF_VARS" \
    -drive format=raw,file=fat:rw:"$ESP_DIR" \
    -net none \
    -nographic \
    -serial file:"$DIR/qemu_serial.log" &

QEMU_PID=$!
sleep 15
kill $QEMU_PID || true
wait $QEMU_PID 2>/dev/null || true
echo "QEMU run finished. Serial output captured in qemu_serial.log."
