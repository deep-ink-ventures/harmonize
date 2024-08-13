#!/bin/bash

echo "Running caddy"

caddy=$(lsof -t -i:8555)
if [ -z "$caddy" ]; then
    echo "caddy not running"
    sudo caddy start
else
    echo "caddy already running"
fi

echo "Setting up containers"

dfx stop
# Find process IDs listening on port 4943 (dfx)
dfx=$(lsof -t -i:4943)
# Check if any PIDs were found
if [ -z "$dfx" ]; then
    echo "dfx not running"
else
    # Kill the processes
    kill $dfx && echo "Terminating running dfx instance."
    sleep 3
fi

echo "Running SIWE"

dfx start --clean --background
dfx canister create --all
dfx deploy evm_rpc

read -r -d '' SIWE_PROVIDER_ARGS << EOM
(
    record {
        domain = "127.0.0.1";
        uri = "http://127.0.0.1:5173";
        salt = "my-secret-salt";
        chain_id = opt 1;
        scheme = opt "http";
        statement = opt "Login to the app";
        sign_in_expires_in = opt 300000000000;
        session_expires_in = opt 604800000000000;
        targets = opt vec {
            "$(dfx canister id ic_siwe_provider)";
            "$(dfx canister id harmonize_backend)";
        };
    }
)
EOM

dfx deploy ic_siwe_provider --argument "${SIWE_PROVIDER_ARGS}"

read -r -d '' HARMONIZE_ARGS << EOM
record {
    environment = "local";
    initial_owner = principal "cgd3n-nsqas-3nelm-2u6qs-khybz-lwlm7-oqrg6-4li2t-l56pu-om7f7-2qe";
    ecdsa_key_id = record {
      name = "dfx_test_key";
      curve = variant { secp256k1 };
    };
    networks = vec {};
}
EOM

dfx deploy --argument "${HARMONIZE_ARGS}" -m reinstall harmonize_backend

echo "Sleeping while canister is being initialized"
sleep 5
dfx canister call harmonize_backend get_ethereum_address | tr -d '()"' > harmonize-canister-address.txt
