#!/bin/bash
set -euo pipefail

for user_name in "${SUDO_USER:-}" "${USER:-}" flux; do
    [ -n "${user_name}" ] || continue
    cargo_bin="/home/${user_name}/.cargo/bin"
    if [ -x "${cargo_bin}/cargo" ]; then
        case ":${PATH}:" in
            *":${cargo_bin}:"*) ;;
            *) export PATH="${cargo_bin}:${PATH}" ;;
        esac
        break
    fi
done
