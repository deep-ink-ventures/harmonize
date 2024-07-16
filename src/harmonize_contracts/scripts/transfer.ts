import { ethers } from "hardhat";

const address = "0x5fbdb2315678afecb367f032d93f642f64180aa3";
const blendsafe = "0xacdb138dace341d8438169714b6d101efa625f9c";

const main = async () => {
  const Coin = await ethers.getContractFactory("Coin");
  const coin = await Coin.attach(address);
  await coin.transfer(blendsafe, "1337");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
