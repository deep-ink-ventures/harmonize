import os
from uuid import uuid4
from web3 import Web3
from config import connect, get_default_identities, get_wallet_id, assert_ok, assert_err, assert_fail, get_default_principals


def test_set_owner():
    wallet_id = get_wallet_id()

    (owner, new_owner, _) = get_default_principals()

    harmonize = connect(index=0)
    result = harmonize.set_owner(new_owner)
    assert_fail(lambda: harmonize.set_owner(new_owner))

    harmonize = connect(index=1)
    result = harmonize.set_owner(owner)
    assert_fail(lambda: harmonize.set_owner(owner))
