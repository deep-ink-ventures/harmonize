"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
require("@nomiclabs/hardhat-ethers");
require("@nomiclabs/hardhat-waffle");
require("@typechain/hardhat");
// import "solidity-coverage";
const config = {
    solidity: "0.8.20",
    paths: {
        sources: "./src/contracts",
        tests: "./test",
        cache: "./cache",
        artifacts: "./artifacts"
    },
    typechain: {
        outDir: "typechain",
        target: "ethers-v5"
    },
    networks: {
        hardhat: {},
        ganache: {
            url: "http://127.0.0.1:8545",
        },
    },
};
exports.default = config;
