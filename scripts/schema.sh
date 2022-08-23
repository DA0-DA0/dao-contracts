START_DIR=$(pwd)

cd contracts/cw-dao-core
cargo run --example schema

cd "$START_DIR"

for f in ./contracts/voting-modules/*
do
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done

for f in ./contracts/proposal-modules/*
do
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done

for f in ./contracts/staking-rewards/*
do
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done

for f in ./contracts/utils/*
do
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done

