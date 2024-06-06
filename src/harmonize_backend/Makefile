CANISTER = harmonize_backend

.DEFAULT_GOAL := did

.PHONY: wasm
wasm:
	cargo build --release --target wasm32-unknown-unknown --package $(CANISTER)

.PHONY: did
did: wasm
	candid-extractor ../../target/wasm32-unknown-unknown/release/${CANISTER}.wasm > ${CANISTER}.did
