/* Autogenerated file. Do not edit manually. */
/* tslint:disable */
/* eslint-disable */
import {
  Contract,
  ContractFactory,
  ContractTransactionResponse,
  Interface,
} from "ethers";
import type {
  Signer,
  AddressLike,
  ContractDeployTransaction,
  ContractRunner,
} from "ethers";
import type { NonPayableOverrides } from "../../../common";
import type {
  Endpoint,
  EndpointInterface,
} from "../../../src/contracts/Endpoint";

const _abi = [
  {
    inputs: [
      {
        internalType: "address",
        name: "_harmonize",
        type: "address",
      },
    ],
    stateMutability: "nonpayable",
    type: "constructor",
  },
  {
    inputs: [],
    name: "BalanceDecreased",
    type: "error",
  },
  {
    inputs: [],
    name: "DepositAmountZero",
    type: "error",
  },
  {
    inputs: [],
    name: "DepositFailed",
    type: "error",
  },
  {
    inputs: [],
    name: "EndpointZeroAddress",
    type: "error",
  },
  {
    inputs: [],
    name: "HarmonizeAddressZero",
    type: "error",
  },
  {
    inputs: [],
    name: "ReceivedAmountZero",
    type: "error",
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "sender",
        type: "address",
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "recipient",
        type: "bytes32",
      },
      {
        indexed: true,
        internalType: "address",
        name: "token",
        type: "address",
      },
      {
        indexed: false,
        internalType: "uint256",
        name: "amount",
        type: "uint256",
      },
    ],
    name: "DepositErc20",
    type: "event",
  },
  {
    anonymous: false,
    inputs: [
      {
        indexed: true,
        internalType: "address",
        name: "sender",
        type: "address",
      },
      {
        indexed: true,
        internalType: "bytes32",
        name: "recipient",
        type: "bytes32",
      },
      {
        indexed: false,
        internalType: "uint256",
        name: "amount",
        type: "uint256",
      },
    ],
    name: "DepositEth",
    type: "event",
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "recipient",
        type: "bytes32",
      },
      {
        internalType: "address",
        name: "token",
        type: "address",
      },
      {
        internalType: "uint256",
        name: "amount",
        type: "uint256",
      },
    ],
    name: "depositErc20",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function",
  },
  {
    inputs: [
      {
        internalType: "bytes32",
        name: "recipient",
        type: "bytes32",
      },
    ],
    name: "depositEth",
    outputs: [],
    stateMutability: "payable",
    type: "function",
  },
  {
    inputs: [],
    name: "harmonize",
    outputs: [
      {
        internalType: "address",
        name: "",
        type: "address",
      },
    ],
    stateMutability: "view",
    type: "function",
  },
] as const;

const _bytecode =
  "0x608034608657601f61046638819003918201601f19168301916001600160401b03831184841017608b57808492602094604052833981010312608657516001600160a01b038116908190036086578015607557600080546001600160a01b0319169190911790556040516103c490816100a28239f35b633e489ca960e11b60005260046000fd5b600080fd5b634e487b7160e01b600052604160045260246000fdfe608080604052600436101561001357600080fd5b60003560e01c9081638c07c0551461034857508063be056b28146101165763da63d7b61461004057600080fd5b60203660031901126101115734156101005760008080803460018060a01b038254165af13d156100fb573d67ffffffffffffffff81116100e55760405190610092601f8201601f19166020018361036c565b8152600060203d92013e5b156100d457604051348152600435907f76dc0ee3258a13329858db8aa068de1965106c72cd8a45b1e57243d5f084c00960203392a3005b6379cacff160e01b60005260046000fd5b634e487b7160e01b600052604160045260246000fd5b61009d565b635d66204960e11b60005260046000fd5b600080fd5b34610111576060366003190112610111576024356001600160a01b03811690819003610111576044358115610337578015610100576000546040516370a0823160e01b81526001600160a01b03909116600482018190529091602083602481875afa9283156102b857600093610303575b50604051916323b872dd60e01b8352336004840152602483015260448201526020816064816000875af19081156102b8576000916102c4575b50156100d4576000546040516370a0823160e01b81526001600160a01b03909116600482015290602082602481865afa9182156102b857600092610281575b50808210610270578082039180831161025a571461024957604051908152600435907f925d325361fe5d43da232768b65ef53fa0a19f747793617feda1c0dc64b9445b60203392a4005b630c757ab160e41b60005260046000fd5b634e487b7160e01b600052601160045260246000fd5b63a54d597f60e01b60005260046000fd5b90916020823d6020116102b0575b8161029c6020938361036c565b810103126102ad57505190836101ff565b80fd5b3d915061028f565b6040513d6000823e3d90fd5b6020813d6020116102fb575b816102dd6020938361036c565b810103126102f757519081151582036102ad5750836101c0565b5080fd5b3d91506102d0565b90926020823d60201161032f575b8161031e6020938361036c565b810103126102ad5750519184610187565b3d9150610311565b63d6f10fd360e01b60005260046000fd5b34610111576000366003190112610111576000546001600160a01b03168152602090f35b90601f8019910116810190811067ffffffffffffffff8211176100e55760405256fea264697066735822122001981f88164323ce3a8edf80e7ffd80318637d21d42e0e4cacbb2d2f9c4cb02864736f6c634300081a0033";

type EndpointConstructorParams =
  | [signer?: Signer]
  | ConstructorParameters<typeof ContractFactory>;

const isSuperArgs = (
  xs: EndpointConstructorParams
): xs is ConstructorParameters<typeof ContractFactory> => xs.length > 1;

export class Endpoint__factory extends ContractFactory {
  constructor(...args: EndpointConstructorParams) {
    if (isSuperArgs(args)) {
      super(...args);
    } else {
      super(_abi, _bytecode, args[0]);
    }
  }

  override getDeployTransaction(
    _harmonize: AddressLike,
    overrides?: NonPayableOverrides & { from?: string }
  ): Promise<ContractDeployTransaction> {
    return super.getDeployTransaction(_harmonize, overrides || {});
  }
  override deploy(
    _harmonize: AddressLike,
    overrides?: NonPayableOverrides & { from?: string }
  ) {
    return super.deploy(_harmonize, overrides || {}) as Promise<
      Endpoint & {
        deploymentTransaction(): ContractTransactionResponse;
      }
    >;
  }
  override connect(runner: ContractRunner | null): Endpoint__factory {
    return super.connect(runner) as Endpoint__factory;
  }

  static readonly bytecode = _bytecode;
  static readonly abi = _abi;
  static createInterface(): EndpointInterface {
    return new Interface(_abi) as EndpointInterface;
  }
  static connect(address: string, runner?: ContractRunner | null): Endpoint {
    return new Contract(address, _abi, runner) as unknown as Endpoint;
  }
}
