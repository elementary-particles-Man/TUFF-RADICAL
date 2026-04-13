#!/bin/bash
# TUFF-RADICAL (T-RAD): TUFF-Xwin Fusion Build Script
set -e

XWIN_SRC="/media/flux/THPDOC/Develop/TUFF-Xwin"
OVERLAY_DIR="$(cd "$(dirname "$0")/../../overlay" && pwd)"
BIN_DIR="${OVERLAY_DIR}/usr/local/bin"

echo "--- T-RAD: Fusing with TUFF-Xwin (Waybroker) ---"

if [ ! -d "$XWIN_SRC" ]; then
    echo "[FATAL] TUFF-Xwin source directory not found at $XWIN_SRC"
    exit 1
fi

echo "[INFO] Compiling TUFF-Xwin in release mode for Sovereign performance..."
(cd "$XWIN_SRC" && cargo build --workspace --release)

echo "[INFO] Copying TUFF-Xwin binaries to T-RAD overlay..."
mkdir -p "$BIN_DIR"
for bin in compd displayd lockd sessiond watchdog waylandd; do
    if [ -f "$XWIN_SRC/target/release/$bin" ]; then
        cp "$XWIN_SRC/target/release/$bin" "$BIN_DIR/tuff-xwin-$bin"
        chmod +x "$BIN_DIR/tuff-xwin-$bin"
        echo "  -> Installed tuff-xwin-$bin"
    fi
done

echo "[INFO] Copying TUFF-Xwin shell tools & profiles..."
cp "$XWIN_SRC"/scripts/tuff-xwin-*.sh "$BIN_DIR/"
for script in "$BIN_DIR"/tuff-xwin-*.sh; do
    mv "$script" "${script%.sh}"
    chmod +x "${script%.sh}"
done

# Copy profiles
mkdir -p "${OVERLAY_DIR}/etc/tuff-xwin/profiles"
cp "$XWIN_SRC"/profiles/*.json "${OVERLAY_DIR}/etc/tuff-xwin/profiles/" || true

echo "[SUCCESS] TUFF-Xwin Fusion Complete."
