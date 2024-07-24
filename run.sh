dfx stop
# Find process IDs listening on port 4943 (dfx)
dfx=$(lsof -t -i:4943)
# Check if any PIDs were found
if [ -z "$dfx" ]; then
    echo "dfx not running."
else
    # Kill the processes
    kill $dfx && echo "Terminating running dfx instance."
    sleep 3
fi

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

read -r -d '' ENDPOINT_31337 < <(cat src/harmonize_contracts/endpoint-address-31337.txt)
read -r -d '' ENDPOINT_31338 < <(cat src/harmonize_contracts/endpoint-address-31338.txt)

read -r -d '' HARMONIZE_ARGS << EOM
record {
    environment = "local";
    initial_owner = principal "o2sw6-57jbr-pe6nk-ifnaq-gbd3b-tnubo-3epmh-lcnyb-cwtr7-ahlos-iae";
    ecdsa_key_id = record {
      name = "dfx_test_key";
      curve = variant { secp256k1 };
    };
    networks = vec {
        record {
            0 = 31337;
            1 = record {
                last_scraped_block_number = 0: nat;
                rpc_services = variant {
                  Custom = record {
                    chainId = 31337 : nat64;
                    services = vec { record { url = "https://localhost:8555"; headers = null } };
                  }
                };
                rpc_service = variant {
                  Custom = record {
                    url = "https://localhost:8555";
                    headers = null;
                  }
                };
                get_logs_address = vec { "${ENDPOINT_31337}" };
                block_tag = variant { Latest = null };
            };
        };
        record {
            0 = 31338;
            1 = record {
                last_scraped_block_number = 0: nat;
                rpc_services = variant {
                  Custom = record {
                    chainId = 31338 : nat64;
                    services = vec { record { url = "https://localhost:8556"; headers = null } };
                  }
                };
                get_logs_address = vec { "${ENDPOINT_31338}" };
                rpc_service = variant {
                  Custom = record {
                    url = "https://localhost:8556";
                    headers = null;
                  }
                };
                block_tag = variant { Latest = null };
            };
        };
    };
}
EOM

dfx deploy --argument "${HARMONIZE_ARGS}" -m reinstall harmonize_backend
