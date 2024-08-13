import os
import json
from time import sleep
from uuid import uuid4
from web3 import Web3
from eth_account.messages import encode_defunct
from config import w3, connect, get_default_identities, get_endpoint_abi, get_wallet_id, assert_ok, assert_err, assert_fail, get_default_principals, get_ganache_dev_accounts, unwrap_ok, unwrap_value, get_coin_address, get_coin_abi, wait_for_next_update, get_w3
from ic import Principal

# The tests in this suite are designed to be run in order, as they depend on the canister state of the previous tests.
# The "owner" is the canister controller, and the "users" are the other principals interacting with the app.

def principal_to_bytes32(principal: Principal) -> bytes:
    assert len(principal.bytes) == 29, "Principal should be 29 bytes"
    return principal.bytes.rjust(32, b'\0')

def bytes32_to_principal(bytes32: bytes) -> Principal:
    assert len(bytes32) == 32, "Bytes32 should be 32 bytes"
    return Principal(bytes=bytes32[3:])

def test_principal_to_bytes32():
    (_, user_a, user_b) = get_default_principals()
    bytes32_a = principal_to_bytes32(user_a)
    assert str(bytes32_to_principal(bytes32_a)) == str(user_a), "Principal should be reversible"
    bytes32_b = principal_to_bytes32(user_b)
    assert str(bytes32_to_principal(bytes32_b)) == str(user_b), "Principal should be reversible"

def test_get_endpoint_address():
    harmonize = connect(index=0)

    # We don't have a value to compare, so just check that the response is ok
    response = harmonize.get_endpoint_address(31337)
    unwrap_value(response)

    response = harmonize.get_endpoint_address(31338)
    unwrap_value(response)

    response = harmonize.get_ethereum_address()
    unwrap_value(response)

def test_deposit_er20():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Get the coin contract
    coin_address = get_coin_address()
    assert Web3.is_address(coin_address), "Invalid EVM address"
    coin = w3.eth.contract(address=coin_address, abi=get_coin_abi())

    # Connect as user A
    harmonize = connect(index=1)

    # We can get the EVM address from the canister
    endpoint_address = w3.to_checksum_address(unwrap_value(harmonize.get_endpoint_address(chain_id)))
    assert Web3.is_address(endpoint_address), "Invalid EVM address"
    endpoint = w3.eth.contract(address=endpoint_address, abi=get_endpoint_abi())

    # Make sure that our balance is 0
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(user_a.bytes, chain_id, coin_address))
    assert user_a_coin_balance == "0", "User A's balance should be 0"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(user_b.bytes, chain_id, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Approve 100 coins to be spent by the harmonize canister
    tx_hash = coin.functions.approve(endpoint_address, 100).transact({'from': account_a.address})
    w3.eth.wait_for_transaction_receipt(tx_hash)

    # Deposit 100 coins into the harmonize canister
    tx_hash = endpoint.functions.depositErc20(principal_to_bytes32(user_a), coin_address, 100).transact({'from': account_a.address})
    w3.eth.wait_for_transaction_receipt(tx_hash)

    # Sleep for a bit to allow the transaction to be processed
    wait_for_next_update(chain_id)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_a), chain_id, coin_address))
    assert user_a_coin_balance == "100", "User A's balance should be 100"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_b), chain_id, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

def test_transfer_erc20():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Get the coin contract
    coin_address = get_coin_address(chain_id)
    assert Web3.is_address(coin_address), "Invalid EVM address"
    coin_address = w3.to_checksum_address(coin_address)

    # Connect as user A
    harmonize = connect(index=1)

    # Make sure that our balance is 100
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_a), chain_id, coin_address))
    assert user_a_coin_balance == "100", "User A's balance should be 100"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_b), chain_id, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Transfer 50 coins from user A to user B
    response = harmonize.transfer_erc20(str(user_a), str(user_b), chain_id, coin_address, "50")
    assert_ok(response)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_a), chain_id, coin_address))
    assert user_a_coin_balance == "50", "User A's balance should be 50"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_b), chain_id, coin_address))
    assert user_b_coin_balance == "50", "User B's balance should be 50"

def test_deposit_eth():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Connect as user A
    harmonize = connect(index=1)

    # We can get the EVM address from the canister
    endpoint_address = w3.to_checksum_address(unwrap_value(harmonize.get_endpoint_address(chain_id)))
    assert Web3.is_address(endpoint_address), "Invalid EVM address"
    endpoint = w3.eth.contract(address=endpoint_address, abi=get_endpoint_abi())

    # Make sure that our balance is 0
    user_a_coin_balance = unwrap_value(harmonize.get_eth_balance(user_a.bytes, chain_id))
    assert user_a_coin_balance == "0", "User A's balance should be 0"
    user_b_coin_balance = unwrap_value(harmonize.get_eth_balance(user_b.bytes, chain_id))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Deposit 1 ETH into the harmonize canister
    one_eth = w3.to_wei(1, 'ether')
    tx_hash = endpoint.functions.depositEth(principal_to_bytes32(user_a)).transact({'from': account_a.address, 'value': one_eth})
    w3.eth.wait_for_transaction_receipt(tx_hash)

    # Sleep for a bit to allow the transaction to be processed
    wait_for_next_update(chain_id)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_a), chain_id))
    assert user_a_coin_balance == str(one_eth), f"User A's balance should be {one_eth}"
    user_b_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_b), chain_id))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

def test_transfer_eth():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Connect as user A
    harmonize = connect(index=1)

    # We can get the EVM address from the canister
    endpoint_address = w3.to_checksum_address(unwrap_value(harmonize.get_endpoint_address(chain_id)))
    assert Web3.is_address(endpoint_address), "Invalid EVM address"

    # Make sure that our balance is 100
    one_eth = w3.to_wei(1, 'ether')
    user_a_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_a), chain_id))
    assert user_a_coin_balance == str(one_eth), f"User A's balance should be {one_eth}"
    user_b_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_b), chain_id))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Transfer 50 coins from user A to user B
    one_half_eth = w3.to_wei(0.5, 'ether')
    response = harmonize.transfer_eth(str(user_a), str(user_b), chain_id, str(one_half_eth))
    assert_ok(response)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_a), chain_id))
    assert user_a_coin_balance == str(one_half_eth), f"User A's balance should be {one_half_eth}"
    user_b_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_b), chain_id))
    assert user_b_coin_balance == str(one_half_eth), f"User B's balance should be {one_half_eth}"

def test_withdraw_erc20():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Connect as user B
    harmonize = connect(index=2)

    # Get the coin contract
    coin_address = get_coin_address(chain_id)
    coin = w3.eth.contract(address=coin_address, abi=get_coin_abi())

    # Make sure that our balance is 50
    amount = 50
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_a), chain_id, coin_address))
    assert user_a_coin_balance == str(amount), f"User A's balance should be {amount}"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_b), chain_id, coin_address))
    assert user_b_coin_balance == str(amount), f"User B's balance should be {amount}"

    # Transfer 50 coins from user A to user B
    response = harmonize.withdraw_erc20(account_b.address, chain_id, coin_address, str(amount))
    assert_ok(response)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_a), chain_id, coin_address))
    assert user_a_coin_balance == str(amount), f"User A's balance should be {amount}"
    user_b_coin_balance = unwrap_value(harmonize.get_erc20_balance(str(user_b), chain_id, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Give ganache some time to process the transaction
    sleep(2)

    # Check that the balance has been updated on chain as expected
    user_b_coin_balance_on_chain = coin.functions.balanceOf(account_b.address).call()
    assert user_b_coin_balance_on_chain == amount, f"User B's balance should be {amount} on chain"

def test_withdraw_eth():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    chain_id = 31337

    # Connect as user B
    harmonize = connect(index=2)

    # Make sure that our balance is 50
    one_half_eth = w3.to_wei(0.5, 'ether')
    user_a_coin_balance = unwrap_value(harmonize.get_eth_balance(str(user_a), chain_id))
    assert user_a_coin_balance == str(one_half_eth), f"User A's balance should be {one_half_eth}"

    user_b_coin_balance_on_chain = w3.eth.get_balance(account_b.address)
    print("user_b_coin_balance_on_chain", user_b_coin_balance_on_chain)

    # Transfer 50 coins from user A to user B
    one_quarter_eth = w3.to_wei(0.25, 'ether')
    response = harmonize.withdraw_eth(account_b.address, chain_id, str(one_quarter_eth))
    assert_ok(response)

    # Give ganache some time to process the transaction
    sleep(2)