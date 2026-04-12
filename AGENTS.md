# AGENTS.md (TUFF-RADICAL)

## User language
- The user reads Japanese only.
- Write explanations, handoff notes, roadmap summaries, and design discussions in Japanese unless explicitly asked otherwise.

## First files to read
1. `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/HANDOFF.md`
2. `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/ROADMAP.md`
3. `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/README.md`

## Repository identity
- Repo path: `/run/media/flux/THPDOC/Develop/TUFF-RADICAL`
- Remote: `https://github.com/elementary-particles-Man/TUFF-RADICAL.git`
- Main branch: `main`

## Repository split policy
- This repository is the bare-metal / Pure Rust OS line.
- Do not turn this repository itself into the Linux distro repository.
- The future Linux distro repo must live under this tree, at a lower path, to avoid sibling-repo sprawl.
- Current planned local home for the distro repo: `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro`

## Product boundary
- `TUFF-RADICAL`:
  Pure Rust bare-metal OS, low-level kernel and hardware experiments.
- `TUFF Linux Distro`:
  Separate Linux distribution effort, only planned here for now.
- `TUFF-Xwin`:
  Sibling repository at `/run/media/flux/THPDOC/Develop/TUFF-Xwin`, intended as an optional package/session layer for the future distro, not the base image itself.

## Build and verification notes
- Bare-metal kernel verification command:
  - `cd /run/media/flux/THPDOC/Develop/TUFF-RADICAL/TUFF-RADICAL-KERNEL && cargo build`
- Current `.gitignore` intentionally excludes:
  - `TUFF-RADICAL-KERNEL/target/`
  - QEMU serial/output logs
  - `logs/`

## Current strategic decisions
- Keep the bare-metal OS and Linux distro as separate products.
- Prefer Debian Stable first for the distro bootstrap unless a later technical reason overturns it.
- Do not make AVX, AVX-512, Vulkan, or custom compression code mandatory in distro v1.
- Use TUFF-Xwin as an optional install-on-top package set, not as a required base component.
- Treat "any PC" as a later expansion target; initial realistic distro scope is x86_64 Intel/AMD with BIOS+UEFI.

## Current progress snapshot
- `TUFF-RADICAL` public GitHub repo already exists and is pushed.
- Repo naming has been unified to `TUFF-RADICAL`.
- Root `README.md` and `.gitignore` are in place.
- A Linux distro roadmap has been written at `projects/tuff-linux-distro/ROADMAP.md`.
- The roadmap already identifies:
  - what is reusable
  - what is unnecessary for distro v1
  - what is missing
  - a phased roadmap to public beta

## Immediate next candidates
- Create the initial distro repo skeleton under `projects/tuff-linux-distro/`
- Add the first `mmdebstrap` recipe
- Define package layering:
  - `tuff-base`
  - `tuff-perf`
  - `tuff-recovery`
  - later `tuff-xwin-*`
- Freeze the v1 hardware support matrix in writing
- Define how TUFF-Xwin packaging will land on top of the distro without entering the base image
