# TUFF-RADICAL: Unified Boot Parameters (Surgical Logos)
# These parameters are shared between Live-Build and Installed GRUB to ensure consistency.

# 1. Performance & Latency
TUFF_PERF_PARAMS="mitigations=off pcie_aspm=off threadirqs nowatchdog rcupdate.rcu_expedited=1 tsc=reliable nvme_core.default_ps_max_latency_us=0 amd_pstate=active transparent_hugepage=madvise zswap.enabled=0 cgroup_disable=rdma,misc page_alloc.shuffle=1"

# 2. USB & Boot Stability (Fix for error -110)
TUFF_USB_PARAMS="usbcore.autosuspend=-1 usbcore.old_scheme_first=1 usbcore.initial_descriptor_timeout=20 pci=noaer random.trust_cpu=on"

# 3. Graphics & Hardware Enablement
TUFF_GFX_PARAMS="video=efifb:off iommu=pt"

# Full unified command line
TUFF_CMDLINE="${TUFF_PERF_PARAMS} ${TUFF_USB_PARAMS} ${TUFF_GFX_PARAMS}"
