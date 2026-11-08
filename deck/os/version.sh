#!/usr/bin/env bash
# deck/os/version.sh – the deck's build version, ONE formula (#423): `<workspace version>.r<commit count>.g<short sha>[.dirty]`,
# e.g. 0.1.0.r3093.g8eccc753. update.sh puts it into the update bundle AND passes it to the binaries' build as
# DECK_BUILD_VERSION (deck-native's build.rs → `deck_native::BUILD`, drawn top right on every deck, always); os/build.sh
# does the same for the ISO. build.rs computes the same string from git itself when the env is not set (a plain PC build);
# deck/native/tests/build_version.rs checks the two agree. Prints `dev` outside a git checkout.
set -euo pipefail
deck=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
v=$(sed -n 's/^version = "\(.*\)"/\1/p' "$deck/Cargo.toml" | head -1)
count=$(git -C "$deck" rev-list --count HEAD 2>/dev/null) || { echo dev; exit 0; }
version="$v.r$count.g$(git -C "$deck" rev-parse --short HEAD)"
[ -z "$(git -C "$deck" status --porcelain -- . 2>/dev/null)" ] || version="$version.dirty"
echo "$version"
