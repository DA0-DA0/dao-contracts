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

download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm
	wget https://github.com/public-awesome/cw-nfts/releases/download/v0.18.0/cw721_base.wasm -O artifacts/cw721_base.wasm
	echo 'ba81e10d053814f1dfbb92f20f77ce1cccf64b27b639db9e2afa8ab5d6ea3cf7  artifacts/cw721_base.wasm' | sha256sum --check

workspace-optimize:
    #!/bin/bash
    if [[ $(uname -m) == 'arm64' ]] || [ $(uname -m) == 'aarch64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/arm64 \
            cosmwasm/optimizer-arm64:0.16.1; \
    elif [[ $(uname -m) == 'x86_64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/amd64 \
            cosmwasm/optimizer:0.16.1; fi
