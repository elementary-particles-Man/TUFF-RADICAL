# TUFF-RADICAL: skel .profile
# Automatically launch TUFF-Xwin (Waybroker) upon login on TTY1

if [ "$(tty)" = "/dev/tty1" ]; then
    echo "--- T-RAD Sovereign Executive: Initiating TUFF-Xwin ---"
    
    # Auto-recover or start the default host-wayland profile
    if command -v tuff-xwin-autostart >/dev/null 2>&1; then
        exec tuff-xwin-autostart
    else
        # Fallback if autostart script is not available
        export TUFF_XWIN_PREFIX=/usr/local
        exec tuff-xwin-start host-wayland
    fi
fi
