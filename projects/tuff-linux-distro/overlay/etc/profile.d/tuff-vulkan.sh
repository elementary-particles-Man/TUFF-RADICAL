#!/bin/sh
# TUFF-RADICAL: Performance / Vulkan State Bridge
# Ensures that userland (Browsers/X11) can see the compute state flags set by tuff-vulkan-init.service

VULKAN_STATE="/run/tuff-vulkan-state"

if [ -f "$VULKAN_STATE" ]; then
    . "$VULKAN_STATE"
    export TUFF_VULKAN_OFFLOAD TUFF_INTEL_COMPUTE_ACTIVE
fi

# Resource limits for large shader compilation (Browsers/IDE)
ulimit -l unlimited 2>/dev/null || true
ulimit -n 1048576 2>/dev/null || true
