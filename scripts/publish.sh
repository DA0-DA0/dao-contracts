#!/usr/bin/env bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

function print_usage() {
  echo "Usage: $0 [-h|--help]"
  echo "Publishes crates to crates.io."
}

if [ $# = 1 ] && { [ "$1" = "-h" ] || [ "$1" = "--help" ] ; }
then
    print_usage
    exit 1
fi

START_DIR=$(pwd)

# ${f    <-- from variable f
#   ##   <-- greedy front trim
#   *    <-- matches anything
#   /    <-- until the last '/'
#  }
# <https://stackoverflow.com/a/3162500>

cd contracts/dao-core
cargo publish --dry-run
cd "$START_DIR"

for f in ./contracts/voting/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

for f in ./contracts/proposal/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

for f in ./contracts/staking/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

for f in ./contracts/pre-propose/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

for f in ./contracts/external/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

for f in ./packages/*
do
  echo "publishing ${f##*/}"
  cd "$f"
  cargo publish --dry-run
  cd "$START_DIR"
done

echo "Everything is published!"

VERSION=$(grep -A1 "\[workspace.package\]" Cargo.toml | awk -F'"' '/version/ {print $2}');
git tag v"$VERSION"
git push origin v"$VERSION"
