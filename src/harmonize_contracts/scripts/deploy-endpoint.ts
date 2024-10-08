// @ts-ignore
import { ethers } from "hardhat";
import fs from "fs";

const main = async () => {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying contracts with account:", deployer.address);

  const harmonizeCanister = fs.readFileSync("../../harmonize-canister-address.txt", "utf-8").trim();
  console.log("Harmonize canister address:", harmonizeCanister);
  const network = await ethers.provider.getNetwork();
  const Endpoint = await ethers.getContractFactory("Endpoint");
  const endpoint = await Endpoint.deploy(harmonizeCanister);

  await endpoint.deployed();

  const filename = `endpoint-address-${network.chainId}.txt`

  console.log("Endpoint address:", endpoint.address);
  console.log(`Storing ${filename}`);
  fs.writeFileSync(filename, endpoint.address);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
