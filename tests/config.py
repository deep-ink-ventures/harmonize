import json
from time import sleep
from uuid import uuid4
from web3 import Web3
from ic import Principal
from ic.canister import Canister
from ic.client import Client
from ic.identity import Identity
from ic.agent import Agent
from mnemonic import Mnemonic

w3 = Web3()

def get_coin_address():
    with open('../src/harmonize_contracts/coin-address.txt', 'r') as file:
        return file.read().strip()

def get_coin_abi():
    with open('../src/harmonize_contracts/artifacts/src/contracts/Coin.sol/Coin.json', 'r') as file:
        return json.loads(file.read())['abi']

def get_ganache_dev_keys():
    return [
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
    ]

def get_ganache_dev_accounts():
    return [w3.eth.account.from_key(key) for key in get_ganache_dev_keys()]


def get_ganache_dev_addresses():
    return [account.address for account in get_ganache_dev_accounts()]

def get_wallet_id():
    return f'test_{str(uuid4())[0:8]}'


def get_id(container):
    return "bkyz2-fmaaa-aaaaa-qaaaq-cai"


def get_default_identities():
    return [
        Identity.from_seed("dumb crucial heart army senior rubber tomorrow uncover brown upgrade road start"),
        Identity.from_seed("sad tiger kite quote erupt auction apple sight barely utility adult reason"),
        Identity.from_seed("turkey enroll pride credit mistake toast speak million report phrase eye margin")
    ]

def get_default_principals():
    return [_.sender().to_str() for _ in get_default_identities()]


def get_agent(identity=None, index=None):
    if index is None:
        index = 0
    if identity is None:
        identity = get_default_identities()[index]
    return Agent(identity, Client(url="http://127.0.0.1:4943"))

def assert_ok(res):
    assert isinstance(res, list), 'Expected list'
    assert 'Ok' in res[0], f'Result not ok: {res}'

def assert_some(res):
    assert isinstance(res, list), 'Expected list'
    assert 'Some' in res[0], 'Result not some'

def assert_err(res, error=""):
    assert isinstance(res, list), 'Expected list'
    assert 'Err' in res[0], 'Result not err'
    if error:
        assert res[0]['Err'] == error, 'Error does not match'

def unwrap_ok(res):
    assert_ok(res)
    return res[0]['Ok']

def unwrap_err(res):
    assert_err(res)
    return res[0]['Err']

def unwrap_some(res):
    assert_some(res)
    return res[0]['Some']

def unwrap_value(res):
    assert isinstance(res, list), 'Expected list'
    return res[0]

def assert_fail(fn):
    fail = False
    try:
        fn()
    except:
        fail = True

    if not fail:
        assert False, 'Expected exception'

def connect(agent=None, identity=None, index=None):
    if agent is None:
        agent = get_agent(identity=identity, index=index)
    return Canister(
        agent=agent,
        canister_id=get_id("harmonize_backend"),
        candid=open("../src/harmonize_backend/harmonize_backend.did").read()
    )

def wait_for_next_update(chain_id=31337):
    harmonize = connect(index=0)
    block_number = harmonize.get_last_scraped_block(chain_id)
    while True:
        current_block_number = harmonize.get_last_scraped_block(chain_id)
        if current_block_number != block_number:
            break
        sleep(1)
