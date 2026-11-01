#!/usr/bin/env bash
# deck/os/update.sh – a remote update for the decks on the LAN (#237): `npm run deck:update` (Git Bash / WSL / Linux).
#   1. the Linux binaries in Docker (the ISO build's container and cargo volume, DECK_BINARIES_ONLY=1) → deck/os/out/bin
#   2. `deck-update bundle` on this PC: the binaries + deck/os/rootfs's deck paths + the scenes, signed with the dev key
#      (~/.scutoons/deck-update.key, made on first use; its public half is in rootfs/usr/share/deck/update-keys)
#      → deck/os/out/update (manifest.json, manifest.json.sig, bundle-<build>.tar.gz)
#   3. `deck-update serve` on port 7790 – decks on the dev channel find it by their scan and update within ~2 minutes
#      (`deck-update now` on the deck at once). Ctrl+C stops it.
# Flags: --no-build (bundle the binaries already in out/bin) · --serve (only serve the last bundle) · --no-serve ·
#        --reboot (the decks reboot instead of restarting services) · --notes "text". Details: deck/os/README.md "Updates".
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
deck=$(cd "$here/.." && pwd)
out=$here/out
build=1; bundle=1; serve=1; extra=()
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) build=0 ;;
    --serve) build=0; bundle=0 ;;
    --no-serve) serve=0 ;;
    --reboot) extra+=(--reboot) ;;
    --notes) extra+=(--notes "$2"); shift ;;
    *) echo "unknown flag $1 (see the header of deck/os/update.sh)"; exit 2 ;;
  esac
  shift
done

if [ "$build" = 1 ]; then
  DECK_BINARIES_ONLY=1 bash "$here/build.sh"
fi
# the host's own deck-update (signs and serves; a native build on Windows)
cargo build --release -q --manifest-path "$deck/Cargo.toml" -p deck-update
tool="$deck/target/release/deck-update"
[ -x "$tool" ] || tool="$tool.exe"

if [ "$bundle" = 1 ]; then
  v=$(sed -n 's/^version = "\(.*\)"/\1/p' "$deck/Cargo.toml" | head -1)
  version="$v.r$(git -C "$deck" rev-list --count HEAD).g$(git -C "$deck" rev-parse --short HEAD)"
  [ -z "$(git -C "$deck" status --porcelain -- . 2>/dev/null)" ] || version="$version.dirty"
  "$tool" bundle --bin "$out/bin" --rootfs "$here/rootfs" --scenes "$deck/stage/scenes" --out "$out/update" \
    --version "$version" --channel dev ${extra[@]+"${extra[@]}"}
fi

if [ "$serve" = 1 ]; then
  "$tool" serve "$out/update"
fi
