config := env_var_or_default('CONFIG', '`pwd`/ci/configs/ci.yaml')
admin_addr := env_var_or_default('ADMIN_ADDR', 'juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg')

build:
	cargo build

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

gen: build gen-schema gen-typescript

gen-schema:
	./scripts/schema.sh

gen-typescript:
	rm -rf types/contracts # Clear out any old or invalid state.
	yarn --cwd ./types install --frozen-lockfile
	yarn --cwd ./types build
	yarn --cwd ./types codegen

integration-test: deploy-local optimize
	RUST_LOG=info CONFIG={{config}} cargo integration-test

integration-test-dev test_name="": 
	SKIP_CONTRACT_STORE=true RUST_LOG=info CONFIG='{{`pwd`}}/ci/configs/local.yaml' cargo integration-test {{test_name}} 

bootstrap-dev: deploy-local optimize
	RUST_LOG=info CONFIG={{config}} cargo run bootstrap-env

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
		ghcr.io/cosmoscontracts/juno:v9.0.0 /opt/setup_and_run.sh {{admin_addr}}

download-deps:
	mkdir -p artifacts target
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw20_base.wasm -O artifacts/cw20_base.wasm
	wget https://github.com/CosmWasm/cw-plus/releases/latest/download/cw4_group.wasm -O artifacts/cw4_group.wasm

optimize:
	cargo install cw-optimizoor || true
	cargo cw-optimizoor Cargo.toml
