pub mod calculate_result;
pub mod submit_result;

use std::{fmt, str::FromStr};
use ethers_core::types::{H160, U256};
use ic_cdk::println;
use crate::{
    types::H160Ext,
    chain_fusion::{
    evm_rpc::LogEntry,
    LogSource,
}, signer::keccak256, state::{mutate_network_state, mutate_state, read_state}};


// because we deploy the canister with topics only matching
// NewJob events we can safely assume that the event is a NewJob.
// let new_job_event = NewJobEvent::from(event);
// this calculation would likely exceed an ethereum blocks gas limit
// but can easily be calculated on the IC
// let result = fibonacci(20);
// we write the result back to the evm smart contract, creating a signature
// on the transaction with chain key ecdsa and sending it to the evm via the
// evm rpc canister
// submit_result(result.to_string(), new_job_event.job_id).await;
// println!("Successfully ran job #{:?}", &new_job_event.job_id);

pub fn handle_transfer_event(network_id: u32, event: TransferEvent) {
    let evm_address = read_state(|s| s.evm_address.clone()).unwrap();
    if event.to == evm_address {
        println!("Wallet {} deposited {} of {}/{}", event.from.to_repr(), event.amount, network_id, event.token.to_repr());
        mutate_state(|s| {
            if let Err(e) = s.wallets.create_and_credit(event.from, network_id, event.token, event.amount) {
                println!("Error crediting wallet: {:?}", e);
            }
        })
    }
}

pub async fn job(network_id: u32, event_source: LogSource, event: LogEntry) {
    mutate_network_state(network_id, |s| s.record_processed_log(event_source.clone()));
    let event = match Event::try_from(event) {
        Ok(event) => event,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };
    match event {
        Event::Transfer(transfer_event) => {
            println!("Transfer event: {:?}", transfer_event);
            handle_transfer_event(network_id, transfer_event);
        }
    }
    println!("Successfully ran job");
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NewJobEvent {
    pub job_id: U256,
}

impl fmt::Debug for NewJobEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NewJobEvent")
            .field("job_id", &self.job_id)
            .finish()
    }
}

impl From<LogEntry> for NewJobEvent {
    fn from(entry: LogEntry) -> NewJobEvent {
        // we expect exactly 2 topics from the NewJob event.
        // you can read more about event signatures [here](https://docs.alchemy.com/docs/deep-dive-into-eth_getlogs#what-are-event-signatures)
        let job_id =
            U256::from_str_radix(&entry.topics[1], 16).expect("the token id should be valid");

        NewJobEvent { job_id }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Transfer(TransferEvent),
}

impl TryFrom<LogEntry> for Event {
    type Error = String;

    fn try_from(entry: LogEntry) -> Result<Event, Self::Error> {
        match entry.topics[0].as_str() {
            _topic0 if _topic0 == TransferEvent::topic().as_str() => {
                let event = TransferEvent::try_from(entry)?;
                Ok(Event::Transfer(event))
            }
            _ => Err(format!("Unknown event signature: {}", entry.topics[0].as_str())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransferEvent {
    pub token: H160,
    pub from: H160,
    pub to: H160,
    pub amount: U256,
}

impl TransferEvent {
    pub fn signature() -> &'static str {
        "Transfer(address,address,uint256)"
    }
    pub fn topic() -> String {
        let topic = hex::encode(keccak256(Self::signature().as_bytes()));
        format!("0x{}", topic)
    }
}

pub fn parse_address_from_topic(topic: &str) -> Result<H160, String> {
    if topic.len() != 66 {
        return Err(format!("Invalid topic length: {}", topic.len()));
    }
    let slice = &topic[26..];
    H160::from_str(&topic[26..])
        .map_err(|_| format!("Failed to parse address from topic: {}", slice))
}

impl TryFrom<LogEntry> for TransferEvent {
    type Error = String;

    fn try_from(entry: LogEntry) -> Result<TransferEvent, Self::Error> {
        if entry.topics.len() != 3 {
            return Err(format!("Expected exactly 3 topics, got {:?}", entry.topics));
        }
        let token = entry.address.parse().map_err(|_| "Invalid address".to_string())?;
        let from: H160 = parse_address_from_topic(&entry.topics[1])?;
        let to: H160 = parse_address_from_topic(&entry.topics[2])?;
        let amount = U256::from_str_radix(&entry.data, 16)
            .map_err(|e| e.to_string())?;
        Ok(TransferEvent { token, from, to, amount })
    }
}