#!/bin/bash
# TUFF-RADICAL: Write Release Manifest
set -e

DISTRO_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
VM_IMG="${DISTRO_DIR}/out/images/vm/tuff-vm-stable-amd64-minbase.raw"
TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
RELEASE_DIR="${DISTRO_DIR}/out/release/${TIMESTAMP}"
mkdir -p "$RELEASE_DIR"

echo "--- TUFF Linux Distro: Generating Release Manifest ---"

# 1. イメージのコピー (シンボリックリンクでも可だが、リリース用にはコピー)
cp "$VM_IMG" "${RELEASE_DIR}/$(basename "$VM_IMG")"

# 2. チェックサムの生成
sha256sum "${RELEASE_DIR}/$(basename "$VM_IMG")" > "${RELEASE_DIR}/SHA256SUMS"

# 3. リリース・マニフェストの書き出し
cat <<EOF > "${RELEASE_DIR}/manifest.json"
{
  "project": "TUFF-RADICAL",
  "component": "distro-bootstrap",
  "version": "v1-bootstrap",
  "timestamp": "${TIMESTAMP}",
  "channel": "bootstrap",
  "artifacts": [
    {
      "name": "$(basename "$VM_IMG")",
      "type": "raw-vm-image",
      "arch": "amd64",
      "sha256": "$(sha256sum "$VM_IMG" | awk '{print $1}')"
    }
  ],
  "base": "Debian 13 (Trixie)"
}
EOF

# 4. パッケージリストのコピー
cp "${DISTRO_DIR}/packages/tuff-base/manifest.txt" "${RELEASE_DIR}/tuff-base.manifest"

echo "--- Release Manifest Written: ${RELEASE_DIR} ---"
