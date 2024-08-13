#!/bin/bash

read -r -d '' ENDPOINT_31337 < <(cat src/harmonize_contracts/endpoint-address-31337.txt)
read -r -d '' ENDPOINT_31338 < <(cat src/harmonize_contracts/endpoint-address-31338.txt)

read -r -d '' NETWORK_CONFIG << EOM
record {
    rpc_services = opt variant {
      Custom = record {
        chainId = 31337 : nat64;
        services = vec { record { url = "https://localhost:8555"; headers = null } };
      }
    };
    rpc_service = opt variant {
      Custom = record {
        url = "https://localhost:8555";
        headers = null;
      }
    };
    get_logs_address = opt vec { "${ENDPOINT_31337}" };
    last_scraped_block_number = opt 0: opt nat;
    block_tag = opt variant { Latest = null };
}
EOM

dfx canister call harmonize_backend set_network_config "(31337, ${NETWORK_CONFIG})"

read -r -d '' NETWORK_CONFIG << EOM
record {
    rpc_services = opt variant {
      Custom = record {
        chainId = 31338 : nat64;
        services = vec { record { url = "https://localhost:8556"; headers = null } };
      }
    };
    rpc_service = opt variant {
      Custom = record {
        url = "https://localhost:8556";
        headers = null;
      }
    };
    get_logs_address = opt vec { "${ENDPOINT_31338}" };
    last_scraped_block_number = opt 0: opt nat;
    block_tag = opt variant { Latest = null };
}
EOM

dfx canister call harmonize_backend set_network_config "(31338, ${NETWORK_CONFIG})"
