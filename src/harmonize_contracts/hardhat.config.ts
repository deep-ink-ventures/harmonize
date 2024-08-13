import { HardhatUserConfig } from "hardhat/config";
import "@nomiclabs/hardhat-ethers";
import "@nomiclabs/hardhat-waffle";
import "@typechain/hardhat";
// import "solidity-coverage";

const config: HardhatUserConfig = {
  solidity: {
    version: "0.8.26", // Specify the version of Solidity you're using
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
        details: {
          yul: true,
          yulDetails: {
            stackAllocation: true
          }
        }
      },
      viaIR: true
    }
  },
  paths: {
    sources: "./src/contracts",
    tests: "./test",
    cache: "./cache",
    artifacts: "./artifacts"
  },
  networks: {
    hardhat: {
        chainId: 31337
    },
    ganache: {
        url: "http://127.0.0.1:8545",
        chainId: 31337,
    },
    custard: {
        url: "http://127.0.0.1:8546",
        chainId: 31338,
    }
  },
};

export default config;
