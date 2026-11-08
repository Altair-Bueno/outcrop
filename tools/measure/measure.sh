#!/usr/bin/env bash
# Usage: measure.sh <outcrop checkout> [label]
# Measures the view-heavy workload against the given outcrop checkout:
#   code size   .text of thumbv7em-none-eabi binaries (opt-level 3, fat LTO), minus an empty binary
#   fuel        wasmtime fuel (operators executed) per extra feed+poll round (32 rounds minus 16), and values reported
#   native      formation size, peak stack (stack painting) and ns per extra round on the host
set -euo pipefail
OUTCROP=$(cd "$1" && pwd)
LABEL=${2:-$(basename "$OUTCROP")}
HERE=$(cd "$(dirname "$0")" && pwd)
RUN=$(mktemp -d "${TMPDIR:-/tmp}/outcrop-measure.XXXXXX")
trap 'rm -rf "$RUN"' EXIT
cp -R "$HERE/workload" "$HERE/size" "$HERE/instr" "$HERE/stack" "$RUN/"
sed "s#@OUTCROP@#$OUTCROP#" "$HERE/Cargo.toml.in" > "$RUN/Cargo.toml"
export CARGO_TARGET_DIR="$HERE/target/$LABEL"
BIN=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin
cd "$RUN"

cargo build -q --release -p size --target thumbv7em-none-eabi \
  --config 'target.thumbv7em-none-eabi.rustflags=["-C","link-arg=--entry=_start","-C","link-arg=--gc-sections"]'
cargo rustc -q --release -p workload --target wasm32-unknown-unknown --crate-type cdylib
cargo build -q --release -p instr -p stack

text() { "$BIN/llvm-size" -A "$CARGO_TARGET_DIR/thumbv7em-none-eabi/release/$1" | awk '$1==".text"{print $2}'; }
EMPTY=$(text size-empty)
echo "== $LABEL"
echo "code size (thumbv7em .text bytes, minus empty binary)"
printf "  builder\t%s\n  macro\t%s\n" $(( $(text size-builder) - EMPTY )) $(( $(text size-macro) - EMPTY ))
echo "wasm fuel per extra round (wasmtime, operators executed), and reports"
"$CARGO_TARGET_DIR/release/instr" "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/workload.wasm"
echo "native aarch64"
"$CARGO_TARGET_DIR/release/stack"
