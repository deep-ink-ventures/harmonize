import { HardhatUserConfig } from "hardhat/config";
import "@nomiclabs/hardhat-ethers";
import "@nomiclabs/hardhat-waffle";
import "@typechain/hardhat";
// import "solidity-coverage";

const config: HardhatUserConfig = {
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

export default config;
