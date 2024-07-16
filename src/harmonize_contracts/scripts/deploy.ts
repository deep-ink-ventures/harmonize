// @ts-ignore
import { ethers } from "hardhat";
import fs from "fs";

const main = async () => {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with the account:", deployer.address);

  const initialSupply = ethers.utils.parseUnits("1000", 18); // 1000 tokens

  const Coin = await ethers.getContractFactory("Coin");
  const coin = await Coin.deploy(initialSupply);

  await coin.deployed();

  console.log("Coin address:", coin.address);
  fs.writeFileSync("coin-address.txt", coin.address);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
