// @ts-ignore
import { ethers } from "hardhat";
import fs from "fs";

const main = async () => {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with account:", deployer.address);

  const initialSupply = ethers.utils.parseUnits("1000", 18); // 1000 tokens
  const network = await ethers.provider.getNetwork();

  const Coin = await ethers.getContractFactory("Coin");
  const coin = await Coin.deploy(initialSupply);

  await coin.deployed();

  const filename = `coin-address-${network.chainId}.txt`

  console.log("Coin address:", coin.address);
  console.log(`Storing ${filename}`);
  fs.writeFileSync(filename, coin.address);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
