import os
import json
from time import sleep
from uuid import uuid4
from web3 import Web3
from eth_account.messages import encode_defunct
from config import w3, connect, get_default_identities, get_endpoint_abi, get_wallet_id, assert_ok, assert_err, assert_fail, get_default_principals, get_ganache_dev_accounts, unwrap_ok, unwrap_value, get_coin_address, get_coin_abi, wait_for_next_update, get_w3

# The tests in this suite are designed to be run in order, as they depend on the canister state of the previous tests.
# The "owner" is the canister controller, and the "users" are the other principals interacting with the app.

def _test_set_owner():
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