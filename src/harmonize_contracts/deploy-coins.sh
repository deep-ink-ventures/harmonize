#!/bin/bash

# Deploy a dummy ERC20 token to our two local nodes.
# Each command produces an output file containing the
# contract address at coin-address-{chainId}.txt.
npx hardhat run --network ganache scripts/deploy-coin.ts
npx hardhat run --network custard scripts/deploy-coin.ts

# Deploy a dummy ERC20 token to our two local nodes.
# Each command produces an output file containing the
# contract address at endpoint-address-{chainId}.txt.
npx hardhat run --network ganache scripts/deploy-endpoint.ts
npx hardhat run --network custard scripts/deploy-endpoint.ts
