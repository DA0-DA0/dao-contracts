#!/bin/ash
# shellcheck shell=dash
# See https://www.shellcheck.net/wiki/SC2187
#
# Entry point for cosmwasm/optimizer:0.16.1 and cosmwasm/optimizer-arm64:0.16.1,
# used by `just workspace-optimize` and .github/workflows/release-contracts.yml
# through `--entrypoint /code/scripts/optimize.sh`.
#
# This is the image's own /usr/local/bin/optimize.sh with two changes:
#
# 1. wasm-opt gets --one-caller-inline-max-function-size=15. By default binaryen
#    inlines every non-exported function that has a single caller, regardless
#    of its size. Serde-derived deserializers form long chains of such
#    functions, so they were merged into the contract entry points and produced
#    functions with 103 to 118 locals. cosmwasm-vm >= 2.2.10 (wasmvm >= 2.2.9,
#    used by THORChain) rejects contracts with a function that declares more
#    than 100 locals. `just check-wasm-limits` verifies the result.
# 2. The project directory defaults to "." because `--entrypoint` clears the
#    image's default CMD (".").
#
# Re-derive this script from the image whenever the optimizer version changes.
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

export PATH="$PATH:/root/.cargo/bin"

# Debug toolchain and default Rust version
rustup toolchain list
cargo --version

# Prepare artifacts directory for later use
mkdir -p artifacts

# Delete previously built artifacts. Those can exist if the image is called
# with a cache mounted to /target. In cases where contracts are removed over time,
# old builds in cache should not be contained in the result of the next build.
rm -f /target/wasm32-unknown-unknown/release/*.wasm

# Ensure we get at most one argument and this is a directory (the path to the
# Cargo project to be built).
if [ "$#" -gt 1 ] || ! [ -d "${1:-.}" ]; then
  echo "Usage: $0 [DIRECTORY]" >&2
  exit 1
fi
PROJECTDIR="${1:-.}"
echo "Building project $(realpath "$PROJECTDIR") ..."
(
  cd "$PROJECTDIR"
  /usr/local/bin/bob
)

echo "Optimizing artifacts ..."
for WASM in /target/wasm32-unknown-unknown/release/*.wasm; do
  [ -e "$WASM" ] || continue # https://superuser.com/a/519493

  OUT_FILENAME=$(basename "$WASM")
  echo "Optimizing $OUT_FILENAME ..."
  # --signext-lowering is needed to support blockchains runnning CosmWasm < 1.3. It can be removed eventually
  wasm-opt -Os --signext-lowering --one-caller-inline-max-function-size=15 "$WASM" -o "artifacts/$OUT_FILENAME"
done

echo "Post-processing artifacts..."
(
  cd artifacts

  if test -n "$(find . -maxdepth 1 -name '*.wasm' -print -quit)"; then
    sha256sum -- *.wasm | tee checksums.txt
  else
    echo "Warn: No .wasm file built. Check your build configuration in Cargo.toml."
  fi
)

echo "Done."
