#!/usr/bin/env bash
# deck/os/build.sh – build the deck live ISO (class A x86) in Docker: `npm run deck:iso` (Git Bash / WSL / Linux / macOS).
# Result: deck/os/out/scutoonsdeck-<date>-x86_64.iso (+ mkarchiso.log; volume label SCUTOONSDECK). Named volumes keep the cargo target and the pacman
# cache between builds (scutoons-deck-work, scutoons-deck-pkg; `docker volume rm` them for a cold build).
# Env: DECK_KERNEL=linux-rt|linux · DECK_SKIP_CARGO=1 · CARGO_BUILD_JOBS (fewer = less memory) · DECK_IMAGE (archlinux:latest) · DECK_OUT (default deck/os/out;
# another folder when the last ISO is still open – a writer tool holds it and mkarchiso cannot replace it). Details: deck/os/README.md.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
deck=$(cd "$here/.." && pwd)
out=${DECK_OUT:-$here/out}
mkdir -p "$out"; out=$(cd "$out" && pwd)   # docker wants an absolute path (a relative DECK_OUT is a volume name to it)
if command -v cygpath >/dev/null 2>&1; then deck_m=$(cygpath -m "$deck"); out_m=$(cygpath -m "$out"); else deck_m=$deck; out_m=$out; fi

docker version >/dev/null 2>&1 || { echo "Docker does not answer – start Docker Desktop"; exit 1; }
echo "building the deck ISO in Docker (kernel ${DECK_KERNEL:-linux-rt}) – the first run downloads ~1 GB"
MSYS_NO_PATHCONV=1 docker run --rm --privileged \
  -e DECK_KERNEL -e DECK_SKIP_CARGO -e DECK_TRACE -e DECK_BINARIES_ONLY -e CARGO_BUILD_JOBS \
  -v "$deck_m:/src:ro" \
  -v scutoons-deck-work:/work \
  -v scutoons-deck-pkg:/var/cache/pacman/pkg \
  -v "$out_m:/out" \
  "${DECK_IMAGE:-archlinux:latest}" bash /src/os/container/build.sh
