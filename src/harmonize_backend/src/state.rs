use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use candid::{CandidType, Deserialize, Nat, Principal};
use ethers_core::types::{H160, U256};
use ic_cdk::api::management_canister::ecdsa::{ecdsa_public_key, EcdsaKeyId, EcdsaPublicKeyArgument};

use crate::chain_fusion::evm_rpc::{LogEntry, BlockTag, RpcService, RpcServices};
use crate::chain_fusion::evm_signer;
use crate::chain_fusion::job::events::{DepositEvent, DepositErc20Event};
use crate::chain_fusion::{LogSource, TaskType};
use crate::wallet::Wallets;
use crate::access_control::AccessControl;

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct NetworkInit {
    pub rpc_services: RpcServices,
    pub rpc_service: RpcService,
    pub last_scraped_block_number: Nat,
    pub get_logs_address: Vec<String>,
    pub block_tag: BlockTag,
}

#[derive(CandidType, Deserialize, Clone, Debug)]
pub struct NetworkMut {
    pub rpc_services: Option<RpcServices>,
    pub rpc_service: Option<RpcService>,
    pub last_scraped_block_number: Option<Nat>,
    pub block_tag: Option<BlockTag>,
    pub get_logs_address: Option<Vec<String>>,
    pub nonce: Option<u128>,
}

impl NetworkMut {
    pub fn new() -> Self {
        NetworkMut {
            rpc_services: None,
            rpc_service: None,
            last_scraped_block_number: None,
            get_logs_address: None,
            block_tag: None,
            nonce: None,
        }
    }
    pub fn into_init(self) -> Option<NetworkInit> {
        Some(NetworkInit {
            rpc_services: self.rpc_services?,
            rpc_service: self.rpc_service?,
            last_scraped_block_number: self.last_scraped_block_number?,
            get_logs_address: self.get_logs_address?,
            block_tag: self.block_tag?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Network {
    pub rpc_services: RpcServices,
    pub rpc_service: RpcService,
    pub last_scraped_block_number: Nat,
    pub last_observed_block_number: Option<Nat>,
    pub last_processed_block_number: Option<Nat>,
    pub logs_to_process: BTreeMap<LogSource, LogEntry>,
    pub get_logs_address: Vec<String>,
    pub processed_logs: BTreeMap<LogSource, LogEntry>,
    pub skipped_blocks: BTreeSet<Nat>,
    pub block_tag: BlockTag,
    pub nonce: U256,
}

impl Network {
    pub fn mutate_with(&mut self, init: NetworkMut) {
        if let Some(rpc_services) = init.rpc_services {
            self.rpc_services = rpc_services;
        }
        if let Some(rpc_service) = init.rpc_service {
            self.rpc_service = rpc_service;
        }
        if let Some(last_scraped_block_number) = init.last_scraped_block_number {
            self.last_scraped_block_number = last_scraped_block_number;
        }
        if let Some(block_tag) = init.block_tag {
            self.block_tag = block_tag;
        }
        if let Some(nonce) = init.nonce {
            self.nonce = U256::from(nonce);
        }
    }

    pub fn record_log_to_process(&mut self, log_entry: &LogEntry) {
        let event_source = log_entry.source();
        assert!(
            !self.logs_to_process.contains_key(&event_source),
            "there must be no two different events with the same source"
        );
        assert!(!self.processed_logs.contains_key(&event_source));

        self.logs_to_process.insert(event_source, log_entry.clone());
    }

    pub fn record_processed_log(&mut self, source: LogSource) {
        let log_entry = match self.logs_to_process.remove(&source) {
            Some(event) => event,
            None => panic!("attempted to run job for an unknown event {source:?}"),
        };

        assert_eq!(
            self.processed_logs.insert(source.clone(), log_entry),
            None,
            "attempted to run job twice for the same event {source:?}"
        );
    }

    pub fn record_skipped_block(&mut self, block_number: Nat) {
        assert!(
            self.skipped_blocks.insert(block_number.clone()),
            "BUG: block {} was already skipped",
            block_number
        );
    }

    pub fn has_logs_to_process(&self) -> bool {
        !self.logs_to_process.is_empty()
    }
}

impl From<NetworkInit> for Network {
    fn from(init: NetworkInit) -> Self {
        Network {
            rpc_services: init.rpc_services,
            rpc_service: init.rpc_service,
            last_scraped_block_number: init.last_scraped_block_number,
            last_observed_block_number: None,
            last_processed_block_number: None,
            logs_to_process: Default::default(),
            get_logs_address: init.get_logs_address,
            processed_logs: Default::default(),
            skipped_blocks: Default::default(),
            nonce: Default::default(),
            block_tag: init.block_tag,
        }
    }
}

pub struct State {
    pub owner: Principal,
    pub wallets: Wallets<Principal>,
    pub access_control: AccessControl,
    pub networks: HashMap<u32, Network>,

    pub active_tasks: HashSet<TaskType>,
    pub get_logs_topics: Option<Vec<Vec<String>>>,
    pub ecdsa_pub_key: Option<Vec<u8>>,
    pub ecdsa_key_id: EcdsaKeyId,
    pub evm_address: Option<H160>,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct Init {
    environment: String,
    initial_owner: Principal,
    ecdsa_key_id: EcdsaKeyId,
    networks: HashMap<u32, NetworkInit>,
}

impl From<Init> for State {
    fn from(init: Init) -> Self {
        let networks = init
            .networks
            .into_iter()
            .map(|(id, init)| (id, init.into()))
            .collect();

        let get_logs_topics = Some(vec![
            vec![DepositEvent::topic()],
            vec![DepositErc20Event::topic()],
        ]);

        State {
            owner: init.initial_owner,
            wallets: Wallets::new(),
            access_control: Default::default(),
            networks,
            get_logs_topics,
            active_tasks: Default::default(),
            ecdsa_key_id: init.ecdsa_key_id,
            ecdsa_pub_key: None,
            evm_address: None,
        }
    }
}

thread_local! {
    pub static STATE: RefCell<Option<State>> = RefCell::default();
}

pub fn read_state<R>(f: impl FnOnce(&State) -> R) -> R {
    STATE.with_borrow(|s| f(s.as_ref().expect("BUG: state is not initialized")))
}

pub fn read_network_state<R>(network_id: u32, f: impl FnOnce(&Network) -> R) -> R {
    read_state(|s| {
        f(&s.networks.get(&network_id).expect("BUG: network is not initialized"))
    })
}

/// Mutates (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    STATE.with_borrow_mut(|s| f(s.as_mut().expect("BUG: state is not initialized")))
}

pub fn mutate_network_state<F, R>(network_id: u32, f: F) -> R
where
    F: FnOnce(&mut Network) -> R,
{
    mutate_state(|s| f(s.networks.get_mut(&network_id).expect("BUG: network is not initialized")))
}

/// Sets the current state to `state`.
pub fn initialize_state(state: State) {
    STATE.set(Some(state));
}

pub fn caller_is_owner() -> bool {
    read_state(|s| s.owner == ic_cdk::caller())
}

pub async fn get_public_key() -> Vec<u8> {
    let key_id = read_state(|s| s.ecdsa_key_id.clone());
    let (key,) = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: [].to_vec(),
        key_id,
    })
    .await
    .expect("failed to get public key");
    key.public_key
}

pub async fn get_eth_address_str() -> String {
    let public_key = get_public_key().await;
    evm_signer::pubkey_bytes_to_address(&public_key)
}

// Public API

pub fn set_owner(new_owner: Principal) {
    if !caller_is_owner() {
        ic_cdk::trap("Only the owner can assign a new owner.");
    }
    mutate_state(|s| {
        s.owner = new_owner;
    });
}

pub fn get_owner() -> Principal {
    read_state(|s| s.owner.clone())
}

pub fn set_network_config(chain_id: u32, network_mut: NetworkMut) {
    if !caller_is_owner() {
        ic_cdk::trap("Only the owner can change the network configuration.");
    }
    mutate_state(|s| {
        let network_id = chain_id;
        if let Some(entry) = s.networks.get_mut(&network_id) {
            entry.mutate_with(network_mut);
        } else {
            let new_network = network_mut.into_init().expect("BUG: invalid network config");
            s.networks.insert(network_id, new_network.into());
        }
    });
}

pub fn get_deposit_address() -> H160 {
    match read_state(|s| s.evm_address.clone()) {
        Some(address) => address,
        None => ic_cdk::trap("Canister not initialized"),
    }
}

pub fn get_last_scraped_block(chain_id: u32) -> Nat {
    read_network_state(chain_id, |n| n.last_scraped_block_number.clone())
}

pub fn get_last_processed_block(chain_id: u32) -> Nat {
    read_network_state(chain_id, |n| {
        n.last_processed_block_number.clone().unwrap_or(Nat::from(0u32))
    })
}