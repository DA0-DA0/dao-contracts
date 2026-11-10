orc_config := env_var_or_default('CONFIG', '`pwd`/ci/configs/cosm-orc/ci.yaml')
test_addrs := env_var_or_default('TEST_ADDRS', `jq -r '.[].address' ci/configs/test_accounts.json | tr '\n' ' '`)
gas_limit := env_var_or_default('GAS_LIMIT', '10000000')

build:
	cargo build

test:
	cargo test

lint:
	cargo +nightly clippy --all-targets -- -D warnings

gen: build gen-schema

gen-schema:
	./scripts/schema.sh

integration-test: deploy-local workspace-optimize
	RUST_LOG=info CONFIG={{orc_config}} cargo integration-test

test-tube:
    cargo test --features "test-tube"

test-tube-dev: workspace-optimize
    cargo test --features "test-tube"

integration-test-dev test_name="":
	SKIP_CONTRACT_STORE=true RUST_LOG=info CONFIG='{{`pwd`}}/ci/configs/cosm-orc/local.yaml' cargo integration-test {{test_name}}

bootstrap-dev: deploy-local workspace-optimize
	RUST_LOG=info CONFIG={{orc_config}} cargo run bootstrap-env

deploy-local: download-deps
	docker kill cosmwasm || true
	docker volume rm -f junod_data
	# Juno v28.0.2 (wasmvm 2.2.2): earlier alpine/wasmer-4.2.2 images abort the
	# node process during StoreCode (CosmWasm/wasmvm#523). v28 requires the real
	# ujuno denom, so replicate the image's start script with matching fees, and
	# raise the RPC body limit so large optimized artifacts fit one transaction.
	docker run --rm -d --name cosmwasm \
		-e PASSWORD=xxxxxxxxx \
		-e STAKE_TOKEN=ujuno \
		-e GAS_LIMIT={{gas_limit}} \
		-e MAX_BYTES=22020096 \
		-e UNSAFE_CORS=true \
		-e JUNOD_GRPC_ADDRESS=0.0.0.0:9090 \
		-p 1317:1317 \
		-p 26656:26656 \
		-p 26657:26657 \
		-p 9090:9090 \
		--mount type=volume,source=junod_data,target=/root \
		ghcr.io/cosmoscontracts/juno@sha256:256d5e441b9b2decdd0473a06df05be45c6fb067f8454ff3e81adddbc2bdbdc9 \
		sh -c '/opt/setup_junod.sh "$@"; sed -i "s/^max_body_bytes = .*/max_body_bytes = 104857600/" /root/.juno/config/config.toml; junod start --rpc.laddr tcp://0.0.0.0:26657 --minimum-gas-prices 0.0001ujuno --trace' _ {{test_addrs}}

# Third-party contracts used by the test-tube and integration tests. They are
# pinned to fixed releases and verified by checksum: workspace-optimize lists
# them in artifacts/checksums.txt, so check-wasm-limits checks them too, and an
# unpinned upstream release could otherwise break CI.
download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/download/v2.0.0/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/download/v2.0.0/cw4_group.wasm -O artifacts/cw4_group.wasm
	wget https://github.com/public-awesome/cw-nfts/releases/download/v0.18.0/cw721_base.wasm -O artifacts/cw721_base.wasm
	printf '%s  %s\n' \
		4d8e90dd340993033f1b9e8e3a3ee7f8673c582ca9bcdd8c8cf3c7470d6537d5 artifacts/cw20_base.wasm \
		4604a284e209c2fe320f223b9fd29805a0e8f2cf8ea7b01fac28c3efc4ee63f0 artifacts/cw4_group.wasm \
		ba81e10d053814f1dfbb92f20f77ce1cccf64b27b639db9e2afa8ab5d6ea3cf7 artifacts/cw721_base.wasm \
		| sha256sum --check

# Build and optimize every contract into artifacts/ exactly as the release
# workflow does (the source must be mounted at /code for reproducible
# checksums), then check the artifacts against cosmwasm-vm's locals limits.
workspace-optimize: && check-wasm-limits
    #!/bin/bash
    set -euo pipefail
    if [[ $(uname -m) == 'arm64' ]] || [[ $(uname -m) == 'aarch64' ]]; then
        platform=linux/arm64
        image=cosmwasm/optimizer-arm64:0.16.1
    elif [[ $(uname -m) == 'x86_64' ]]; then
        platform=linux/amd64
        image=cosmwasm/optimizer:0.16.1
    else
        echo "unsupported architecture: $(uname -m)" >&2
        exit 1
    fi
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        --platform "$platform" \
        --entrypoint /bin/ash \
        "$image" /code/scripts/optimize.sh

# Fail when an artifact listed in artifacts/checksums.txt (written by the
# optimizer) has a function with more than 100 locals; cosmwasm-vm >= 2.2.10
# rejects such contracts at store time. Runs after every workspace-optimize.
check-wasm-limits:
    node scripts/check-wasm-limits.mjs --checksums artifacts/checksums.txt
