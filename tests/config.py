from uuid import uuid4

from ic import Principal
from ic.canister import Canister
from ic.client import Client
from ic.identity import Identity
from ic.agent import Agent
from mnemonic import Mnemonic


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
    assert 'Ok' in res[0], f'Result not ok: {res}'


def assert_err(res, error=""):
    assert 'Err' in res[0], 'Result not err'
    if error:
        assert res[0]['Err'] == error, 'Error not match'

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
