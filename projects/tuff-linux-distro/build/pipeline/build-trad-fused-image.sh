#!/bin/bash
# T-RAD (TUFF-RADICAL): Sovereign Integrated Fusion Build Pipeline
set -e

# Paths
PIPELINE_DIR="$(cd "$(dirname "$0")" && pwd)"
TRAD_ROOT="$(cd "${PIPELINE_DIR}/../../.." && pwd)"
XWIN_SRC="${XWIN_SRC:-$(cd "${TRAD_ROOT}/../TUFF-Xwin" 2>/dev/null && pwd || echo "")}"
OS_SRC="${OS_SRC:-$(cd "${TRAD_ROOT}/../TUFF-OS" 2>/dev/null && pwd || echo "")}"
OVERLAY_DIR="${TRAD_ROOT}/projects/tuff-linux-distro/overlay"
BIN_DIR="${OVERLAY_DIR}/usr/local/bin"
BOOT_DIR="${OVERLAY_DIR}/boot"

# Fixed build output for reliability
TEMP_TARGET_DIR="/tmp/trad-fusion-build"
mkdir -p "${TEMP_TARGET_DIR}"

echo "================================================================"
echo "   T-RAD SOVEREIGN EXECUTIVE: INTEGRATED FUSION PIPELINE"
echo "================================================================"

# Ensure directories exist
mkdir -p "${BIN_DIR}"
mkdir -p "${BOOT_DIR}"

# 1. Build T-RAD Pure Rust Kernel (x86-64-v4)
echo "[1/4] Building T-RAD Kernel..."
(cd "${TRAD_ROOT}/TUFF-RADICAL-KERNEL" && cargo build --release --target-dir "${TEMP_TARGET_DIR}/kernel")
cp "${TEMP_TARGET_DIR}/kernel/x86_64-unknown-uefi/release/tuff-radical-kernel.efi" "${BOOT_DIR}/tuff-radical-kernel.efi"

# 2. Build TUFF-Xwin (Waybroker Stack)
echo "[2/4] Building TUFF-Xwin Display Stack..."
(cd "${XWIN_SRC}" && cargo build --workspace --release --target-dir "${TEMP_TARGET_DIR}/xwin")

XWIN_BINS=(compd displayd lockd sessiond watchdog waylandd)
for bin in "${XWIN_BINS[@]}"; do
    SRC_BIN="${TEMP_TARGET_DIR}/xwin/release/$bin"
    if [ -f "$SRC_BIN" ]; then
        cp "$SRC_BIN" "${BIN_DIR}/tuff-xwin-$bin"
    else
        echo "[ERROR] Binary $bin NOT FOUND at $SRC_BIN"
        exit 1
    fi
done

# Handle scripts
cp "${XWIN_SRC}"/scripts/tuff-xwin-*.sh "${BIN_DIR}/" || true
for script in "${BIN_DIR}"/tuff-xwin-*.sh; do
    if [ -f "$script" ]; then
        mv "$script" "${script%.sh}"
    fi
done

# 3. Build Fused TUFF-Installer (Network-less, T-RAD Target)
echo "[3/4] Building Fused T-RAD Integrated Installer..."
(cd "${OS_SRC}/TUFF-INSTALLER" && cargo build --release --target-dir "${TEMP_TARGET_DIR}/installer")
cp "${TEMP_TARGET_DIR}/installer/release/tuff-installer" "${BIN_DIR}/tuff-installer"

# 4. Finalizing Overlay & System Hooks
echo "[4/4] Synchronizing T-RAD Sovereign Hooks..."
find "${BIN_DIR}" -type f -exec chmod +x {} +

# Ensure T-RAD Rescue points to the fused installer
cat <<EOF > "${BIN_DIR}/tuff-fusion-deploy"
#!/bin/bash
echo "--- T-RAD SOVEREIGN DEPLOYMENT ENGINE ---"
echo "Initiating direct hardware fusion..."
/usr/local/bin/tuff-installer install --unattended
EOF
chmod +x "${BIN_DIR}/tuff-fusion-deploy"

echo "================================================================"
echo "   FUSION COMPLETE: T-RAD INTEGRATED SYSTEM READY"
echo "================================================================"
rm -rf "${TEMP_TARGET_DIR}"
