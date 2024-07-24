import os
import json
from time import sleep
from uuid import uuid4
from web3 import Web3
from eth_account.messages import encode_defunct
from config import w3, connect, get_default_identities, get_wallet_id, assert_ok, assert_err, assert_fail, get_default_principals, get_ganache_dev_accounts, unwrap_ok, unwrap_value, get_coin_address, get_coin_abi, wait_for_next_update, get_w3

# The tests in this suite are designed to be run in order, as they depend on the canister state of the previous tests.
# The "owner" is the canister controller, and the "users" are the other principals interacting with the app.

def test_set_owner():
    wallet_id = get_wallet_id()

    (owner, new_owner, _) = get_default_principals()

    # Temporarily set the owner to the new owner
    harmonize = connect(index=0)
    response = harmonize.set_owner(new_owner)

    # Attempt to set the owner to the new owner again
    # This should fail because the owner is already set to the new owner
    assert_fail(lambda: harmonize.set_owner(new_owner))

    # Set the owner back to the original owner
    harmonize = connect(index=1)
    response = harmonize.set_owner(owner)

    # Attempt to set the owner to the original owner again
    # This should fail because the owner is already set to the original owner
    assert_fail(lambda: harmonize.set_owner(owner))

def test_get_evm_address():
    harmonize = connect(index=0)
    response = harmonize.get_evm_address()
    # We don't have a value to compare, so just check that the response is ok
    unwrap_value(response)

def test_link_wallet():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    # Connect as user A and request a challenge
    harmonize = connect(index=1)
    response = harmonize.sign_in_challenge(account_a.address)
    message = unwrap_ok(response)

    response = harmonize.has_access(user_a, account_a.address)
    has_access = unwrap_value(response)
    assert not has_access, "User A should not have access to account A"

    # Sign the message with the private key
    message = encode_defunct(text=message)
    signed_message = w3.eth.account.sign_message(message, private_key=account_a.key)
    response = harmonize.sign_in_with_signature(account_a.address, signed_message.signature.hex())
    assert_ok(response)

    # User A should now have access to account A
    response = harmonize.has_access(user_a, account_a.address)
    has_access = unwrap_value(response)
    assert has_access, "User A should have access to account A"

    # User B should not have access to account A
    response = harmonize.has_access(user_b, account_a.address)
    has_access = unwrap_value(response)
    assert not has_access, "User B should not have access to account A"

    # Connect as user B and request a challenge
    harmonize = connect(index=2)
    response = harmonize.sign_in_challenge(account_b.address)
    message = unwrap_ok(response)

    response = harmonize.has_access(user_b, account_b.address)
    has_access = unwrap_value(response)
    assert not has_access, "User B should not have access to account B"

    # Sign the message with the private key
    message = encode_defunct(text=message)
    signed_message = w3.eth.account.sign_message(message, private_key=account_b.key)
    response = harmonize.sign_in_with_signature(account_b.address, signed_message.signature.hex())
    assert_ok(response)

    # User B should now have access to account B
    response = harmonize.has_access(user_b, account_b.address)
    has_access = unwrap_value(response)
    assert has_access, "User B should have access to account B"

    # User A should not have access to account B
    response = harmonize.has_access(user_a, account_b.address)
    has_access = unwrap_value(response)
    assert not has_access, "User A should not have access to account B"


def test_deposit_coins():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    # Instantiate a web3 instance using port 8485 and account A's private key
    # w3 = get_w3(index=0)
    # w3.eth.default_account = account_a

    # Get the coin contract
    coin_address = get_coin_address()
    contract = w3.eth.contract(address=coin_address, abi=get_coin_abi())

    # Connect as user A
    harmonize = connect(index=1)

    # We can get the EVM address from the canister
    evm_address = w3.to_checksum_address(unwrap_value(harmonize.get_evm_address()))
    assert Web3.is_address(evm_address), "Invalid EVM address"

    # Make sure that our balance is 0
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "0", "User A's balance should be 0"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Send 100 coins to the harmonize canister
    tx_hash = contract.functions.transfer(evm_address, 100).transact({'from': account_a.address})
    w3.eth.wait_for_transaction_receipt(tx_hash)

    # Sleep for a bit to allow the transaction to be processed
    wait_for_next_update(31337)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "100", "User A's balance should be 100"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

def test_transfer_coins():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    # Get the coin contract
    coin_address = get_coin_address()
    # contract = w3.eth.contract(address=coin_address, abi=get_coin_abi())
    # w3 = get_w3(index=0)

    # Connect as user A
    harmonize = connect(index=1)

    # We can get the EVM address from the canister
    evm_address = w3.to_checksum_address(unwrap_value(harmonize.get_evm_address()))
    assert Web3.is_address(evm_address), "Invalid EVM address"

    # Make sure that our balance is 100
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "100", "User A's balance should be 100"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Transfer 50 coins from user A to user B
    response = harmonize.transfer(account_a.address, account_b.address, 31337, coin_address, "50")
    assert_ok(response)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "50", "User A's balance should be 50"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "50", "User B's balance should be 50"

def test_withdraw_coins():
    (owner, user_a, user_b) = get_default_principals()
    (account_a, account_b) = get_ganache_dev_accounts()

    # Connect as user B
    harmonize = connect(index=2)

    # We can get the EVM address from the canister
    evm_address = w3.to_checksum_address(unwrap_value(harmonize.get_evm_address()))
    assert Web3.is_address(evm_address), "Invalid EVM address"

    # Send some ETH to the harmonize canister to cover the gas fees
    tx_hash = w3.eth.send_transaction({'from': account_a.address, 'to': evm_address, 'value': w3.to_wei(1, 'ether')})
    w3.eth.wait_for_transaction_receipt(tx_hash)

    # Get the coin contract
    coin_address = get_coin_address()
    contract = w3.eth.contract(address=coin_address, abi=get_coin_abi())

    # Make sure that our balance is 50
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "50", "User A's balance should be 50"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "50", "User B's balance should be 50"

    # Transfer 50 coins from user A to user B
    response = harmonize.withdraw(account_b.address, 31337, coin_address, "50")
    assert_ok(response)

    # Check that the balance has been updated as expected
    user_a_coin_balance = unwrap_value(harmonize.get_balance(account_a.address, 31337, coin_address))
    assert user_a_coin_balance == "50", "User A's balance should be 50"
    user_b_coin_balance = unwrap_value(harmonize.get_balance(account_b.address, 31337, coin_address))
    assert user_b_coin_balance == "0", "User B's balance should be 0"

    # Give ganache some time to process the transaction
    sleep(5)

    # Check that the balance has been updated on chain as expected
    user_b_coin_balance_on_chain = contract.functions.balanceOf(account_b.address).call()
    assert user_b_coin_balance_on_chain == 50, "User B's balance should be 50 on chain"