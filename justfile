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

integration-test: deploy-local
    sleep 10
    docker ps || grep cosmwasm || echo "container not found"
    @echo "sleep more"
    sleep 10
    @echo "Testing Tendermint RPC (26657)..."
    curl -f -s --max-time 5 http://localhost:26657/status > /tmp/rpc_test.json && echo "Tendermint OK" || echo "Tendermint failed"
    @test -f /tmp/rpc_test.json && jq -r '.result.sync_info.latest_block_height' /tmp/rpc_test.json || echo "No block height data"
    @echo "Testing REST API (1317)..."
    curl -f -s --max-time 5 http://localhost:1317/cosmos/base/tendermint/v1beta1/blocks/latest > /tmp/rest_test.json && echo "REST OK" || echo "REST failed"
    @test -f /tmp/rest_test.json && jq -r '.block.header.height' /tmp/rest_test.json || echo "No REST height data"
    @echo "Testing gRPC port (9090) accessibility..."
    timeout 5 bash -c '</dev/tcp/localhost/9090' && echo "gRPC port open" || echo "gRPC port closed"
    @echo ""
    @echo "DEBUG: Container network info"
    docker exec cosmwasm netstat -tlnp 2>/dev/null | head -10 || echo "netstat failed"
    @echo ""
    @echo "DEBUG: Config file content"
    @echo "Using config: {{orc_config}}"
    cat {{orc_config}} || echo "Config file not found"
    @echo ""
    @echo "DEBUG: Container app.toml gRPC config"
    docker exec cosmwasm grep -A 5 "\[grpc\]" /root/.juno/config/app.toml || echo "gRPC config check failed"
    @echo ""
    @echo "Starting integration tests with full output..."
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
	docker run --rm -d --name cosmwasm \
		-e PASSWORD=xxxxxxxxx \
		-e STAKE_TOKEN=ujunox \
		-e GAS_LIMIT={{gas_limit}} \
		-e MAX_BYTES=22020096 \
		-e UNSAFE_CORS=true \
		-p 1317:1317 \
		-p 26656:26656 \
		-p 26657:26657 \
		-p 9090:9090 \
		--mount type=volume,source=junod_data,target=/root \
		ghcr.io/cosmoscontracts/juno:v24.0.0 /opt/setup_and_run.sh {{test_addrs}}

download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm
	wget https://github.com/CosmWasm/cw-nfts/releases/latest/download/cw721_base.wasm -O artifacts/cw721_base.wasm

workspace-optimize:
    #!/bin/bash
    if [[ $(uname -m) == 'arm64' ]] || [ $(uname -m) == 'aarch64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/arm64 \
            cosmwasm/optimizer-arm64:0.17.0; \
    elif [[ $(uname -m) == 'x86_64' ]]; then docker run --rm -v "$(pwd)":/code \
            --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
            --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
            --platform linux/amd64 \
            cosmwasm/optimizer:0.17.0; fi
