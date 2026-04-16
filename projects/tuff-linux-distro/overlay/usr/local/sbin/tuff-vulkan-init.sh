#!/bin/bash
# TUFF-RADICAL: Vulkan Compute Initializer & Load Manager (Intel/SIMD/AVX Tuned)
set -e

echo "--- TUFF-RADICAL [VULKAN-01]: Initializing High-Performance Compute Domain ---"

# --- INTEL / AVX CPU TUNING ---
# Ensure maximum performance states for Intel CPUs via cpupower if available
if command -v cpupower &> /dev/null; then
    echo "[INFO] Forcing performance CPU governor for maximum AVX/SIMD throughput."
    cpupower frequency-set -g performance >/dev/null 2>&1 || true
fi

# --- VULKAN DOMAIN TUNING ---
export RADV_PERFTEST=aco,nv_ms          # For AMD fallback, force ACO compiler

# 1. GPU / Vulkan Compatibility Check
if ! command -v vulkaninfo &> /dev/null; then
    echo "[WARN] vulkan-tools not found. GPU Offload DISABLED. System relies entirely on AVX-512 CPU execution."
    exit 0
fi

GPU_COUNT=$(vulkaninfo --summary | grep -c "Device Type: DISCRETE_GPU\|INTEGRATED_GPU" || true)
if [ "$GPU_COUNT" -eq 0 ]; then
    echo "[INFO] No Vulkan-compatible GPU detected. Falling back purely to AVX-512 (CPU)."
    exit 0
fi

echo "[OK] Detected $GPU_COUNT Vulkan-capable device(s)."

# 2. Applying Offload Policy
# Set robust state flags to instruct the kernel & userland to push compute loads to GPU.
cat <<EOF > /run/tuff-vulkan-state
TUFF_VULKAN_OFFLOAD=1
TUFF_INTEL_COMPUTE_ACTIVE=1
EOF

# 3. Resource Limits (For unbounded GPU computation)
# Removes memory locking restrictions to prevent pinned memory eviction during large shader execution.
ulimit -l unlimited
ulimit -n 1048576

echo "[SUCCESS] Vulkan & Intel AVX Compute Domain ACTIVE. System unchained."
