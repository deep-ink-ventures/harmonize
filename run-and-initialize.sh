#!/bin/bash

pushd src/harmonize_backend
make
popd

./run.sh

pushd src/harmonize_contracts
node_pid=$(lsof -t -i:8545)
if [ -z "$node_pid" ]; then
    echo "node is not running"
    ./run-nodes.sh &
fi
./deploy-coins.sh
popd

./initialize.sh
