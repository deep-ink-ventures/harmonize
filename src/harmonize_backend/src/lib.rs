use std::cell::RefCell;
use std::collections::BTreeMap;
use candid::{CandidType, Deserialize, Principal};
use ethers_core::types::{H160, U256};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Ord, PartialOrd, CandidType, Deserialize)]
pub struct NetworkId (u32);

impl NetworkId {
    pub fn new(id: u32) -> Self {
        NetworkId(id)
    }
    pub fn inner(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Default)]
struct NetworkConfig {
    rpc_endpoints: Vec<String>,
    quorum: u32,
}

type NetworkConfigStore = BTreeMap<NetworkId, NetworkConfig>;

// TODO: move to it's own file
struct Wallet {
    user: Principal,
    balances: BTreeMap<H160, U256>,
}

type WalletStore = BTreeMap<NetworkId, Wallet>;

thread_local! {
    static OWNER: RefCell<Option<Principal>> = RefCell::default();
    static NETWORK_CONFIGS: RefCell<NetworkConfigStore> = RefCell::default();
    static WALLETS: RefCell<WalletStore> = RefCell::default();
}

pub fn caller_is_owner() -> bool {
    OWNER.with_borrow(|owner| {
        *owner == Some(ic_cdk::caller())
    })
}

#[ic_cdk::init]
fn init(_environment: String, initial_owner: Principal) {
    OWNER.with_borrow_mut(|owner| {
        *owner = Some(initial_owner);
    });
}

#[ic_cdk::update]
fn set_owner(new_owner: Principal) {
    if !caller_is_owner() {
        ic_cdk::trap("Only the owner can assign a new owner.");
    }
    OWNER.with_borrow_mut(|owner| {
        *owner = Some(new_owner);
    });
}

#[ic_cdk::update]
fn set_network_config(chain_id: u32, rpc_endpoints: Vec<String>, quorum: u32) {
    if !caller_is_owner() {
        ic_cdk::trap("Only the owner can change the network configuration.");
    }
    NETWORK_CONFIGS.with_borrow_mut(|network_configs| {
        let network_id = NetworkId::new(chain_id);
        if let Some(entry) = network_configs.get_mut(&network_id) {
            entry.rpc_endpoints = rpc_endpoints;
            entry.quorum = quorum;
        } else {
            network_configs.insert(network_id, NetworkConfig {
                rpc_endpoints,
                quorum,
            });
        }
    });
}

#[ic_cdk::update]
fn sync_logs() {
    // Implementation for syncing events
}

#[ic_cdk::query]
fn check_balance(chain_id: NetworkId, address: String, erc20: Option<String>) -> u128 {
    // Implementation for checking balance
    // Return a dummy balance for now
    1000
}

#[ic_cdk::query]
fn check_balance_native(chain_id: NetworkId) -> u128 {
    // Implementation for checking native balance
    // Return a dummy balance for now
    1000
}

// we'll do this later
#[ic_cdk::query]
fn get_challenge(address: String) -> String {
    // Implementation for getting challenge
    // use let (bytes,): (Vec<u8>,) = ic_cdk::api::call(Principal.management_canister(), "raw_rand", ()).await?;

    // todo own file
    // save challenge for 10 blocks for a specific address
    "dummy_challenge".to_string()
}

#[ic_cdk::update]
fn withdraw(signed_challenge: String, erc20: Option<String>, amount: u128) {
    // Implementation for withdrawing ERC20 tokens
}

#[ic_cdk::update]
fn withdraw_native(signed_challenge: String, amount: u128) {
    // Implementation for withdrawing native tokens
}

// Enable Candid export
ic_cdk::export_candid!();