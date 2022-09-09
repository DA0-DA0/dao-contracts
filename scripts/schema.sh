START_DIR=$(pwd)
for f in ./contracts/*
do
  # we've temporarially disabled this one so we can focus on getting
  # v2 out and apply the pre-propose module refactor later.
  if [ "$dir" == "cw-proposal-multiple" ] ; then
    continue;
  fi
  cd "$f"
  CMD="cargo run --example schema"
  eval $CMD
  cd "$START_DIR"
done
