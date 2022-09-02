CONFIG ?= "configs/local.yaml"
ADMIN_ADDR ?= "juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg"

.PHONY: build test lint integration-test bootstrap-dev deploy-local download-deps optimize

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

integration-test: deploy-local optimize
	RUST_LOG=info CONFIG=$(CONFIG) cargo integration-test

bootstrap-dev: deploy-local optimize
	RUST_LOG=info CONFIG=$(CONFIG) cargo run bootstrap-env

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
		ghcr.io/cosmoscontracts/juno:v9.0.0 /opt/setup_and_run.sh $(ADMIN_ADDR)

download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm

optimize:
	cargo install cw-optimizoor || true
	cargo cw-optimizoor Cargo.toml
