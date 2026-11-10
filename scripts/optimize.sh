#!/bin/ash
# shellcheck shell=dash
#
# Runs the cosmwasm/optimizer image's own build script with one extra wasm-opt
# flag. Used by `just workspace-optimize` (and through it by CI and
# .github/workflows/release-contracts.yml) as
#
#   docker run ... --entrypoint /bin/ash cosmwasm/optimizer:0.16.1 /code/scripts/optimize.sh
#
# The flag, --one-caller-inline-max-function-size=15, stops binaryen from
# inlining every function that has a single caller regardless of its size. By
# default serde-derived deserializer chains were merged into the contract entry
# points, producing functions with 103 to 118 locals; cosmwasm-vm >= 2.2.10
# (wasmvm >= 2.2.9, used by THORChain) rejects contracts with a function that
# declares more than 100 locals. `just check-wasm-limits` verifies the result.
#
# The image's /usr/local/bin/optimize.sh is patched at run time instead of
# being copied here, so the build always follows the image in use (including
# after an optimizer upgrade). The script fails if the wasm-opt line it patches
# is not found exactly once. It also removes the previous build's
# artifacts/checksums.txt before building and fails if no contract was built,
# so the locals check never runs against stale or third-party-only artifacts.
set -o errexit -o nounset -o pipefail

FLAG="--one-caller-inline-max-function-size=15"
UPSTREAM=/usr/local/bin/optimize.sh
PATCHED=/tmp/optimize.sh

# The flag goes right before the input file, after the image's own flags.
if [ "$(grep -c '^ *wasm-opt -Os .* "\$WASM" ' "$UPSTREAM")" -ne 1 ]; then
  echo "error: expected exactly one 'wasm-opt -Os ... \"\$WASM\"' line in $UPSTREAM; update scripts/optimize.sh for this optimizer image" >&2
  exit 1
fi
sed "s|^\( *wasm-opt -Os .*\) \"\\\$WASM\" |\1 $FLAG \"\\\$WASM\" |" "$UPSTREAM" > "$PATCHED"
if [ "$(grep -c -- "^ *wasm-opt -Os .* $FLAG \"\\\$WASM\" " "$PATCHED")" -ne 1 ]; then
  echo "error: failed to add $FLAG to the wasm-opt invocation in $UPSTREAM" >&2
  exit 1
fi

# The image's script only writes artifacts/checksums.txt when it finds .wasm
# files, so remove the previous build's copy first. Otherwise a build that
# produced nothing would leave it in place and `just check-wasm-limits` would
# check stale artifacts instead of failing. This runs in the container (as
# root, at /code) because a local artifacts/ directory created by an earlier
# optimizer run is owned by root. Nothing else in artifacts/ is removed: the
# third-party contracts from `just download-deps` live there too.
rm -f artifacts/checksums.txt

# The image's default CMD (".") is replaced by this script's path, so default
# the project directory here.
/bin/ash "$PATCHED" "${1:-.}"

# The image's script only warns when nothing was built, and checksums.txt would
# then list just the third-party contracts from `just download-deps`. Fail
# instead, and remove that checksums.txt so a later `just check-wasm-limits`
# fails too; the image's script empties this directory before building.
if ! ls /target/wasm32-unknown-unknown/release/*.wasm >/dev/null 2>&1; then
  rm -f artifacts/checksums.txt
  echo "error: the optimizer built no contracts" >&2
  exit 1
fi
