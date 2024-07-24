// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract Endpoint {

    error EndpointZeroAddress();
    error DepositAmountZero();
    error DepositFailed();
    error BalanceDecreased();
    error ReceivedAmountZero();
    error HarmonizeAddressZero();

    event DepositEth(
        address indexed sender,
        address indexed recipient,
        uint256 amount
    );

    event DepositErc20(
        address indexed sender,
        address indexed recipient,
        address indexed token,
        uint256 amount
    );

    address public harmonize;

    constructor(address _harmonize) Ownable(msg.sender) {
        require(_harmonize != address(0), HarmonizeAddressZero());
        harmonize = _harmonize;
    }

    function depositEth(bytes32 recipient) external payable {
        require(msg.value > 0, DepositAmountZero());
        (bool status,) = harmonize.call{value: msg.value}("");
        require(status, DepositFailed());
        emit DepositEth(msg.sender, recipient, msg.value);
    }

    function depositErc20(bytes32 recipient, address token, uint256 amount) external {
        require(amount > 0, DepositAmountZero());

        uint256 currentBalance = IERC20(token).balanceOf(harmonize);
        (bool status,) = IERC20(token).transferFrom(msg.sender, harmonize, amount);
        require(status, DepositFailed());

        uint256 newBalance = IERC20(token).balanceOf(harmonize);
        require(newBalance >= currentBalance, BalanceDecreased());

        uint256 amountReceived = newBalance - currentBalance;
        require(amountReceived > 0, ReceivedAmountZero());

        emit DepositErc20(msg.sender, recipient, token, amountReceived);
    }
}
