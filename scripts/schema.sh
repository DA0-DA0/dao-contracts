START_DIR=$(pwd)
for f in ./contracts/*
do
  # we've temporarially disabled this one so we can focus on getting
  # v2 out and apply the pre-propose module refactor later.
  if [ "$f" == "./contracts/cw-proposal-multiple" ] ; then
    continue;
  fi
  echo "generating schema for $f"
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD > /dev/null
  cd "$START_DIR"
done
