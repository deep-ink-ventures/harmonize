/* Autogenerated file. Do not edit manually. */
/* tslint:disable */
/* eslint-disable */
import type {
  BaseContract,
  BigNumberish,
  BytesLike,
  FunctionFragment,
  Result,
  Interface,
  EventFragment,
  AddressLike,
  ContractRunner,
  ContractMethod,
  Listener,
} from "ethers";
import type {
  TypedContractEvent,
  TypedDeferredTopicFilter,
  TypedEventLog,
  TypedLogDescription,
  TypedListener,
  TypedContractMethod,
} from "../../common";

export interface EndpointInterface extends Interface {
  getFunction(
    nameOrSignature: "depositErc20" | "depositEth" | "harmonize"
  ): FunctionFragment;

  getEvent(
    nameOrSignatureOrTopic: "DepositErc20" | "DepositEth"
  ): EventFragment;

  encodeFunctionData(
    functionFragment: "depositErc20",
    values: [BytesLike, AddressLike, BigNumberish]
  ): string;
  encodeFunctionData(
    functionFragment: "depositEth",
    values: [BytesLike]
  ): string;
  encodeFunctionData(functionFragment: "harmonize", values?: undefined): string;

  decodeFunctionResult(
    functionFragment: "depositErc20",
    data: BytesLike
  ): Result;
  decodeFunctionResult(functionFragment: "depositEth", data: BytesLike): Result;
  decodeFunctionResult(functionFragment: "harmonize", data: BytesLike): Result;
}

export namespace DepositErc20Event {
  export type InputTuple = [
    sender: AddressLike,
    recipient: BytesLike,
    token: AddressLike,
    amount: BigNumberish
  ];
  export type OutputTuple = [
    sender: string,
    recipient: string,
    token: string,
    amount: bigint
  ];
  export interface OutputObject {
    sender: string;
    recipient: string;
    token: string;
    amount: bigint;
  }
  export type Event = TypedContractEvent<InputTuple, OutputTuple, OutputObject>;
  export type Filter = TypedDeferredTopicFilter<Event>;
  export type Log = TypedEventLog<Event>;
  export type LogDescription = TypedLogDescription<Event>;
}

export namespace DepositEthEvent {
  export type InputTuple = [
    sender: AddressLike,
    recipient: BytesLike,
    amount: BigNumberish
  ];
  export type OutputTuple = [sender: string, recipient: string, amount: bigint];
  export interface OutputObject {
    sender: string;
    recipient: string;
    amount: bigint;
  }
  export type Event = TypedContractEvent<InputTuple, OutputTuple, OutputObject>;
  export type Filter = TypedDeferredTopicFilter<Event>;
  export type Log = TypedEventLog<Event>;
  export type LogDescription = TypedLogDescription<Event>;
}

export interface Endpoint extends BaseContract {
  connect(runner?: ContractRunner | null): Endpoint;
  waitForDeployment(): Promise<this>;

  interface: EndpointInterface;

  queryFilter<TCEvent extends TypedContractEvent>(
    event: TCEvent,
    fromBlockOrBlockhash?: string | number | undefined,
    toBlock?: string | number | undefined
  ): Promise<Array<TypedEventLog<TCEvent>>>;
  queryFilter<TCEvent extends TypedContractEvent>(
    filter: TypedDeferredTopicFilter<TCEvent>,
    fromBlockOrBlockhash?: string | number | undefined,
    toBlock?: string | number | undefined
  ): Promise<Array<TypedEventLog<TCEvent>>>;

  on<TCEvent extends TypedContractEvent>(
    event: TCEvent,
    listener: TypedListener<TCEvent>
  ): Promise<this>;
  on<TCEvent extends TypedContractEvent>(
    filter: TypedDeferredTopicFilter<TCEvent>,
    listener: TypedListener<TCEvent>
  ): Promise<this>;

  once<TCEvent extends TypedContractEvent>(
    event: TCEvent,
    listener: TypedListener<TCEvent>
  ): Promise<this>;
  once<TCEvent extends TypedContractEvent>(
    filter: TypedDeferredTopicFilter<TCEvent>,
    listener: TypedListener<TCEvent>
  ): Promise<this>;

  listeners<TCEvent extends TypedContractEvent>(
    event: TCEvent
  ): Promise<Array<TypedListener<TCEvent>>>;
  listeners(eventName?: string): Promise<Array<Listener>>;
  removeAllListeners<TCEvent extends TypedContractEvent>(
    event?: TCEvent
  ): Promise<this>;

  depositErc20: TypedContractMethod<
    [recipient: BytesLike, token: AddressLike, amount: BigNumberish],
    [void],
    "nonpayable"
  >;

  depositEth: TypedContractMethod<[recipient: BytesLike], [void], "payable">;

  harmonize: TypedContractMethod<[], [string], "view">;

  getFunction<T extends ContractMethod = ContractMethod>(
    key: string | FunctionFragment
  ): T;

  getFunction(
    nameOrSignature: "depositErc20"
  ): TypedContractMethod<
    [recipient: BytesLike, token: AddressLike, amount: BigNumberish],
    [void],
    "nonpayable"
  >;
  getFunction(
    nameOrSignature: "depositEth"
  ): TypedContractMethod<[recipient: BytesLike], [void], "payable">;
  getFunction(
    nameOrSignature: "harmonize"
  ): TypedContractMethod<[], [string], "view">;

  getEvent(
    key: "DepositErc20"
  ): TypedContractEvent<
    DepositErc20Event.InputTuple,
    DepositErc20Event.OutputTuple,
    DepositErc20Event.OutputObject
  >;
  getEvent(
    key: "DepositEth"
  ): TypedContractEvent<
    DepositEthEvent.InputTuple,
    DepositEthEvent.OutputTuple,
    DepositEthEvent.OutputObject
  >;

  filters: {
    "DepositErc20(address,bytes32,address,uint256)": TypedContractEvent<
      DepositErc20Event.InputTuple,
      DepositErc20Event.OutputTuple,
      DepositErc20Event.OutputObject
    >;
    DepositErc20: TypedContractEvent<
      DepositErc20Event.InputTuple,
      DepositErc20Event.OutputTuple,
      DepositErc20Event.OutputObject
    >;

    "DepositEth(address,bytes32,uint256)": TypedContractEvent<
      DepositEthEvent.InputTuple,
      DepositEthEvent.OutputTuple,
      DepositEthEvent.OutputObject
    >;
    DepositEth: TypedContractEvent<
      DepositEthEvent.InputTuple,
      DepositEthEvent.OutputTuple,
      DepositEthEvent.OutputObject
    >;
  };
}
