pub mod eth_get_logs;
pub mod evm_rpc;
pub mod evm_signer;
pub mod fees;
pub mod guard;
pub mod job;
pub mod eth_call;
pub mod eth_send_raw_transaction;

use std::time::Duration;
use eth_get_logs::scrape_eth_logs_on_all_networks;
use crate::{
    chain_fusion::evm_rpc::LogEntry,
    state::mutate_state
};
use candid::Nat;

// pub const SCRAPING_LOGS_INTERVAL: Duration = Duration::from_secs(3 * 60);
pub const SCRAPING_LOGS_INTERVAL: Duration = Duration::from_secs(30);

pub fn setup_timers() {
    // as timers are synchronous, we need to spawn a new async task to get the public key
    ic_cdk_timers::set_timer(Duration::ZERO, || {
        ic_cdk::spawn(async {
            let public_key = evm_signer::get_public_key().await;
            let evm_address = evm_signer::pubkey_bytes_to_address(&public_key);
            println!("Container EVM address: {:?}", evm_address);
            mutate_state(|s| {
                s.ecdsa_pub_key = Some(public_key);
                s.evm_address = Some(evm_address.parse().expect("address should be valid"));
            });
        })
    });
    // // Start scraping logs almost immediately after the install, then repeat with the interval.
    ic_cdk_timers::set_timer(Duration::from_secs(10), || ic_cdk::spawn(scrape_eth_logs_on_all_networks()));
    ic_cdk_timers::set_timer_interval(SCRAPING_LOGS_INTERVAL, || ic_cdk::spawn(scrape_eth_logs_on_all_networks()));
}

// TODO: Move this to another module
#[derive(Debug, Eq, PartialEq)]
pub enum InvalidStateError {
    InvalidEthereumContractAddress(String),
    InvalidTopic(String),
}

impl LogEntry {
    pub fn source(&self) -> LogSource {
        LogSource {
            transaction_hash: self
                .transactionHash
                .clone()
                .expect("for finalized blocks logs are not pending"),
            log_index: self
                .logIndex
                .clone()
                .expect("for finalized blocks logs are not pending"),
        }
    }
}

/// A unique identifier of the event source: the source transaction hash and the log
/// entry index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogSource {
    pub transaction_hash: String,
    pub log_index: Nat,
}

#[derive(Debug, Hash, Copy, Clone, PartialEq, Eq)]
pub enum TaskType {
    ProcessLogs,
    ScrapeLogs,
}