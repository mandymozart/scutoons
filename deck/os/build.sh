#!/usr/bin/env bash
# deck/os/build.sh – build the deck live ISO (class A x86) in Docker: `npm run deck:iso` (Git Bash / WSL / Linux / macOS).
# Result: deck/os/out/deck-<date>-x86_64.iso (+ mkarchiso.log). Named volumes keep the cargo target and the pacman
# cache between builds (scutoons-deck-work, scutoons-deck-pkg; `docker volume rm` them for a cold build).
# Env: DECK_KERNEL=linux-rt|linux · DECK_SKIP_CARGO=1 · DECK_IMAGE (archlinux:latest). Details: deck/os/README.md.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
deck=$(cd "$here/.." && pwd)
mkdir -p "$here/out"
if command -v cygpath >/dev/null 2>&1; then deck_m=$(cygpath -m "$deck"); out_m=$(cygpath -m "$here/out"); else deck_m=$deck; out_m=$here/out; fi

docker version >/dev/null 2>&1 || { echo "Docker does not answer – start Docker Desktop"; exit 1; }
echo "building the deck ISO in Docker (kernel ${DECK_KERNEL:-linux-rt}) – the first run downloads ~1 GB"
MSYS_NO_PATHCONV=1 docker run --rm --privileged \
  -e DECK_KERNEL -e DECK_SKIP_CARGO -e DECK_TRACE \
  -v "$deck_m:/src:ro" \
  -v scutoons-deck-work:/work \
  -v scutoons-deck-pkg:/var/cache/pacman/pkg \
  -v "$out_m:/out" \
  "${DECK_IMAGE:-archlinux:latest}" bash /src/os/container/build.sh
