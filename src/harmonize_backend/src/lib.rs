use std::cell::RefCell;
use std::collections::{HashMap, BTreeMap};
use candid::{CandidType, Deserialize, Principal};
use chain_fusion::evm_rpc::EVM_RPC;
use ethers_core::types::{H160, U256};

pub mod chain_fusion;
pub mod ecdsa;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone, Ord, PartialOrd, CandidType, Deserialize)]
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

struct Wallet {
    erc20_token_balances: BTreeMap<H160, BTreeMap<NetworkId, U256>>,
}

impl Wallet {

    pub fn fund(&mut self, network_id: NetworkId, address: H160, amount: U256) {
        let balances = self.erc20_token_balances.entry(address).or_insert(BTreeMap::new());
        balances.insert(network_id, amount);
    }

    pub fn withdraw(&mut self, network_id: NetworkId, address: H160, amount: U256) -> Result<U256, String> {
        let balance = self.erc20_token_balances.get_mut(&address).and_then(|balances| {
            balances.get_mut(&network_id)
        });
        if let Some(balance) = balance {
            if *balance >= amount {
                *balance -= amount;
                Ok(amount)
            } else {
                Err("Insufficient balance".to_string())
            }
        } else {
            Err("No balance found".to_string())
        }
    }

    pub fn transfer(&mut self, network_id: NetworkId, from: H160, to: H160, amount: U256) -> Result<(), String> {
        match self.withdraw(network_id, from, amount) {
            Ok(balance) => {
                self.fund(network_id, to, balance);
                Ok(())
            },
            Err(err) => return Err(err),
        }
    }

    pub fn get_balance(&self, network_id: NetworkId, address: H160) -> Option<U256> {
        self.erc20_token_balances.get(&address).and_then(|balances| {
            balances.get(&network_id).cloned()
        })
    }
}

type WalletStore = BTreeMap<Principal, Wallet>;

struct ChallengeKey {
    pub principal: Principal,
    pub network_id: NetworkId,
    pub wallet_address: H160,
}

struct Challange {
    pub message: String,
    pub expires_at: u64, // Block number
}

thread_local! {
    static OWNER: RefCell<Option<Principal>> = RefCell::default();
    static NETWORK_CONFIGS: RefCell<NetworkConfigStore> = RefCell::default();
    static WALLETS: RefCell<WalletStore> = RefCell::default();
    static CHALLENGES: RefCell<HashMap<ChallengeKey, String>> = RefCell::default();
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
    // NOTE: This is an adapted version of the original implementation.
    //       Events are funneled to our custom handler.
    chain_fusion::eth_get_logs::scrape_eth_logs();

}

#[ic_cdk::query]
pub fn get_balance(principal: Principal, network_id: NetworkId, address: String) -> Option<u128> {
    let addr: H160 = address.parse().ok()?;
    WALLETS.with_borrow(|wallets| {
        wallets.get(&principal).and_then(|wallet| {
            wallet.erc20_token_balances.get(&addr).and_then(|balances| {
                balances.get(&network_id).cloned().map(|u| u.as_u128())
            })
        })
    })
}

#[ic_cdk::query]
fn check_balance_native(chain_id: NetworkId) -> u128 {
    // TODO: Initialize the RpcServices
    // EVM_RPC.eth_get_balance(arg0, arg1, arg2, cycles)
    0
}

// we'll do this later
#[ic_cdk::query]
fn get_challenge(address: String) -> String {
    // Implementation for getting challenge
    // use let (bytes,): (Vec<u8>,) = ic_cdk::api::call(Principal.management_canister(), "raw_rand", ()).await?;
    // todo own file
    "dummy_challenge".to_string()

}

// #[ic_cdk::update]
// async fn answer_challenge(wallet_id: String, message: String, signature: String) -> Result<bool, String> {
//     let message = hex::decode(message)
//         .map_err(|_| "Invalid message".to_string())?;
//     let signature = hex::decode(signature)
//         .map_err(|_| "Invalid signature".to_string())?;
//     Ok(ecdsa::is_signature_valid(message, signature, wallet_id, "harmonize").await?)
// }

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
