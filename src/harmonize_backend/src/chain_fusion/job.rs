pub mod safe;

use std::{fmt, str::FromStr};
use ethers_core::types::{H160, U256};
use ic_cdk::println;
use crate::{
    chain_fusion::{
    evm_rpc::LogEntry,
    LogSource,
}, signer::keccak256, state::{mutate_network_state, mutate_state, read_state}, types::H160Ext, wallet::{Erc20, Eth}};


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

use events::*;

pub fn handle_deposit_native_event(network_id: u32, event: DepositEvent) {
    println!("Wallet {} deposited {} of native currency on network {}", event.sender.to_repr(), event.amount, network_id);
    mutate_state(|s| {
        if let Err(e) = s.wallets.credit::<Eth>(event.recipient, &network_id, event.amount) {
            println!("Error crediting wallet: {:?}", e);
        }
    })
}

pub fn handle_deposit_erc20_event(network_id: u32, event: DepositErc20Event) {
    println!("Wallet {} deposited {} of {}/{}", event.sender.to_repr(), event.amount, network_id, event.token.to_repr());
    mutate_state(|s| {
        if let Err(e) = s.wallets.credit::<Erc20>(event.recipient, &(network_id, event.token), event.amount) {
            println!("Error crediting wallet: {:?}", e);
        }
    })
}

pub async fn handle_event(network_id: u32, event_source: LogSource, event: LogEntry) {
    mutate_network_state(network_id, |s| s.record_processed_log(event_source.clone()));
    let event = match Event::try_from(event) {
        Ok(event) => event,
        Err(e) => {
            println!("Error: {:?}", e);
            return;
        }
    };
    match event {
        Event::Deposit(deposit_native_event) => {
            println!("DepositNative event: {:?}", deposit_native_event);
            handle_deposit_native_event(network_id, deposit_native_event);
        },
        Event::DepositErc20(deposit_erc20_event) => {
            println!("DepositERC20 event: {:?}", deposit_erc20_event);
            handle_deposit_erc20_event(network_id, deposit_erc20_event);
        },
    }
}

pub mod events {
    use candid::Principal;
    use ethers_core::types::{H160, U256};
    use crate::{chain_fusion::evm_rpc::LogEntry, signer::keccak256};

    use super::parse_address_from_topic;

    #[derive(Debug, Clone)]
    pub enum Event {
        Deposit(DepositEvent),
        DepositErc20(DepositErc20Event),
    }

    impl TryFrom<LogEntry> for Event {
        type Error = String;

        fn try_from(entry: LogEntry) -> Result<Event, Self::Error> {
            match entry.topics[0].as_str() {
                _topic0 if _topic0 == DepositEvent::topic().as_str() => {
                    let event = DepositEvent::try_from(entry)?;
                    Ok(Event::Deposit(event))
                }
                _topic0 if _topic0 == DepositErc20Event::topic().as_str() => {
                    let event = DepositErc20Event::try_from(entry)?;
                    Ok(Event::DepositErc20(event))
                }
                _ => Err(format!("Unknown event signature: {}", entry.topics[0].as_str())),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct DepositEvent {
        pub sender: H160,
        pub recipient: Principal,
        pub amount: U256,
    }

    impl DepositEvent {
        pub fn signature() -> &'static str {
            "DepositNative(address,uint256)"
        }
        pub fn topic() -> String {
            let topic = hex::encode(keccak256(Self::signature().as_bytes()));
            format!("0x{}", topic)
        }
    }

    impl TryFrom<LogEntry> for DepositEvent {
        type Error = String;

        fn try_from(entry: LogEntry) -> Result<DepositEvent, Self::Error> {
            if entry.topics.len() != 2 {
                return Err(format!("Expected exactly 2 topics, got {:?}", entry.topics));
            }
            let sender: H160 = parse_address_from_topic(&entry.topics[1])?;
            let recipient: Principal = Principal::from_text(&entry.topics[2]).map_err(|e| e.to_string())?;
            let amount = U256::from_str_radix(&entry.data, 16)
                .map_err(|e| e.to_string())?;
            Ok(DepositEvent { sender, recipient, amount })
        }
    }

    #[derive(Debug, Clone)]
    pub struct DepositErc20Event {
        pub sender: H160,
        pub recipient: Principal,
        pub token: H160,
        pub amount: U256,
    }

    impl DepositErc20Event {
        pub fn signature() -> &'static str {
            "DepositERC20(address,address,uint256)"
        }
        pub fn topic() -> String {
            let topic = hex::encode(keccak256(Self::signature().as_bytes()));
            format!("0x{}", topic)
        }
    }

    impl TryFrom<LogEntry> for DepositErc20Event {
        type Error = String;

        fn try_from(entry: LogEntry) -> Result<DepositErc20Event, Self::Error> {
            if entry.topics.len() != 4 {
                return Err(format!("Expected exactly 3 topics, got {:?}", entry.topics));
            }
            let sender: H160 = parse_address_from_topic(&entry.topics[1])?;
            let recipient: Principal = Principal::from_text(&entry.topics[2]).map_err(|e| e.to_string())?;
            let token: H160 = parse_address_from_topic(&entry.topics[3])?;
            let amount = U256::from_str_radix(&entry.data, 16)
                .map_err(|e| e.to_string())?;
            Ok(DepositErc20Event { sender, recipient, token, amount })
        }
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