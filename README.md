# TUFF-RADICAL

TUFF-RADICAL is a Pure Rust bare-metal OS experiment targeting UEFI/QEMU. The current repository centers on a standalone kernel prototype, hardware-facing subsystems, and an accompanying Codex skill used to steer low-level development work.

## Repository Layout

- `TUFF-RADICAL-KERNEL/`: UEFI kernel crate, memory/paging setup, interrupt/GDT wiring, async task executor, GPU command-ring PoC, VirtIO block-device installation simulation, and in-memory ZRAM prototype.
- `skills/SKILL.md`: Development skill definition for the TUFF-RADICAL project.
- `tuff-radical-commander.skill`: Packaged version of the skill for distribution/import.
- `overhaul_docs.py`: Helper script for rewriting markdown documentation to TUFF-RADICAL terminology.

## Quick Start

```bash
cd TUFF-RADICAL-KERNEL
cargo build
./run_qemu.sh
```

## Notes

- Build artifacts and QEMU logs are intentionally git-ignored.
- This codebase is experimental and oriented toward low-level prototyping rather than production deployment.
