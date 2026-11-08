#!/usr/bin/env bash
# deck/os/modules.sh – builds the rig modules a deck ships (#406): walk, stepseq, analog → wasm32, into the folder given as $1
# (default deck/target/x1-modules). The deck loads every *.wasm of DECK_MODULES (/usr/share/deck/modules on the X1).
# Built like the module build service builds a member (deck/build/src/determinism.rs, #344): the same pinned RUSTFLAGS
# (generic cpu, no SIMD, no relaxed SIMD; CARGO_ENCODED_RUSTFLAGS wins over every other source of flags) and the release
# profile of the workspace (fat LTO, one codegen unit, abort on panic) – so the module plays the same bits here as in the
# creator and on the PC. The wasm32 target is the host's (`rustup target add wasm32-unknown-unknown`).
# The test `os_modules_script_pins_the_service_s_flags` (deck/build) keeps the flags below equal to determinism.rs's.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
deck=$(cd "$here/.." && pwd)
dest=${1:-$deck/target/x1-modules}
work=$deck/target/x1-wasm   # not target/wasm-modules (the tests' unpinned build): different flags would rebuild each other's output
mkdir -p "$dest"; dest=$(cd "$dest" && pwd)

unset RUSTFLAGS CARGO_BUILD_RUSTFLAGS CARGO_BUILD_TARGET RUSTC_WRAPPER
export CARGO_ENCODED_RUSTFLAGS=$'-Ctarget-cpu=generic\x1f-Ctarget-feature=-simd128,-relaxed-simd'
cargo build --release --locked -q --target wasm32-unknown-unknown --manifest-path "$deck/Cargo.toml" \
  -p walk-module -p stepseq-module -p analog-module --target-dir "$work"

rm -f "$dest"/*.wasm
for m in walk stepseq analog; do
  install -Dm644 "$work/wasm32-unknown-unknown/release/${m}_module.wasm" "$dest/${m}_module.wasm"
done
ls -l "$dest"/*.wasm
