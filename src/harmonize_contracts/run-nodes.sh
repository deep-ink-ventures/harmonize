#!/bin/bash

# Run two local nodes for development and demos.
# See `hardhat.config.ts`

echo "Running local anvil nodes"
echo ""
echo " - name: ganache"
echo "   chainId: 31337"
echo ""
echo " - name: custard"
echo "   chainId: 31337"
echo ""

anvil --port 8545 --chain-id 31337 &
ANVIL_PID1=$!

anvil --port 8546 --chain-id 31338 &
ANVIL_PID2=$!

echo "PIDs: ${ANVIL_PID1} ${ANVIL_PID2}"

cleanup() {
  echo "Stopping anvil instances..."
  kill $ANVIL_PID1
  kill $ANVIL_PID2
  wait $ANVIL_PID1
  wait $ANVIL_PID2
}

# Trap CTRL-C (SIGINT) and call the cleanup function
trap cleanup SIGINT
wait $ANVIL_PID1
wait $ANVIL_PID2
