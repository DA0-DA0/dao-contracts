orc_config := env_var_or_default('CONFIG', '`pwd`/ci/configs/cosm-orc/ci.yaml')
test_addrs := env_var_or_default('TEST_ADDRS', `jq -r '.[].address' ci/configs/test_accounts.json | tr '\n' ' '`)

build:
	cargo build

test:
	cargo test

lint:
	cargo +nightly clippy --all-targets -- -D warnings

gen: build gen-schema gen-typescript

gen-schema:
	./scripts/schema.sh

gen-typescript:
	git checkout typescript/contracts # Clear out any old or invalid state.
	yarn --cwd ./typescript install --frozen-lockfile
	yarn --cwd ./typescript build
	yarn --cwd ./typescript codegen

integration-test: deploy-local workspace-optimize
	RUST_LOG=info CONFIG={{orc_config}} cargo integration-test

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
		-e GAS_LIMIT=100000000 \
		-e MAX_BYTES=22020096 \
		-e UNSAFE_CORS=true \
		-p 1317:1317 \
		-p 26656:26656 \
		-p 26657:26657 \
		-p 9090:9090 \
		--mount type=volume,source=junod_data,target=/root \
		ghcr.io/cosmoscontracts/juno:v9.0.0 /opt/setup_and_run.sh {{test_addrs}}

download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm
	wget https://github.com/CosmWasm/cw-nfts/releases/latest/download/cw721_base.wasm -O artifacts/cw721_base.wasm

optimize:
	cargo install cw-optimizoor || true
	cargo cw-optimizoor Cargo.toml

workspace-optimize:
	docker run --rm -v "$(pwd)":/code \
		--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		--platform linux/amd64 \
		cosmwasm/workspace-optimizer:0.12.9
