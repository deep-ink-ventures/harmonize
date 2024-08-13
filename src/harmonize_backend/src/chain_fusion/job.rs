pub mod safe;

use std::{fmt, str::FromStr};
use candid::{types::principal, Principal};
use ethers_core::types::{H160, U256};
use ic_cdk::println;
use thiserror::Error;
use crate::{
    chain_fusion::{
    evm_rpc::LogEntry,
    ecdsa::keccak256,
    LogSource,
}, state::{mutate_network_state, mutate_state, read_state}, types::H160Ext, wallet::{Erc20, Eth}};


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

pub fn handle_deposit_eth_event(network_id: u32, event: DepositEthEvent) {
    println!("Wallet {} deposited {} of eth currency on network {}", event.sender.to_repr(), event.amount, network_id);
    mutate_state(|s| {
        if let Err(e) = s.wallets.credit::<Eth>(event.recipient, &network_id, event.amount) {
            println!("Error crediting wallet: {:?}", e);
        }
    })
}

pub fn handle_deposit_erc20_event(network_id: u32, event: DepositErc20Event) {
    println!("Wallet {} deposited {} of {}/{} to {}", event.sender.to_repr(), event.amount, network_id, event.token.to_repr(), event.recipient);
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
        Event::DepositEth(deposit_eth_event) => {
            println!("DepositEth event: {:?}", deposit_eth_event);
            handle_deposit_eth_event(network_id, deposit_eth_event);
        },
        Event::DepositErc20(deposit_erc20_event) => {
            println!("DepositErc20 event: {:?}", deposit_erc20_event);
            handle_deposit_erc20_event(network_id, deposit_erc20_event);
        },
    }
}

#[derive(Error, Debug)]
pub enum ParseEventError {
    #[error("Failed to parse event: {0}")]
    UnknownEventSignature(String),
    #[error("Invalid topics")]
    InvalidTopics,
    #[error("Failed to parse uint")]
    FailedToParseUint,
    #[error("Failed to parse address")]
    FailedToParseAddress,
    #[error("Failed to parse principal")]
    FailedToParsePrincipal,
}

pub fn parse_principal_from_topic(topic: &str) -> Result<Principal, ParseEventError> {
    let topic_bytes = hex::decode(&topic[2..])
        .map_err(|_| ParseEventError::FailedToParsePrincipal)?;
    assert!(topic_bytes.len() == 32, "Principal topic must be 32 bytes");
    let principal_bytes = &topic_bytes[3..];
    Ok(Principal::from_slice(principal_bytes))
}

pub mod events {
    use candid::Principal;
    use ethers_core::types::{H160, U256};
    use crate::{chain_fusion::evm_rpc::LogEntry, chain_fusion::ecdsa::keccak256};

    use super::{parse_address_from_topic, parse_principal_from_topic, ParseEventError};

    #[derive(Debug, Clone)]
    pub enum Event {
        DepositEth(DepositEthEvent),
        DepositErc20(DepositErc20Event),
    }

    impl TryFrom<LogEntry> for Event {
        type Error = ParseEventError;

        fn try_from(entry: LogEntry) -> Result<Event, Self::Error> {
            match entry.topics[0].as_str() {
                _topic0 if _topic0 == DepositEthEvent::topic().as_str() => {
                    let event = DepositEthEvent::try_from(entry)?;
                    Ok(Event::DepositEth(event))
                }
                _topic0 if _topic0 == DepositErc20Event::topic().as_str() => {
                    let event = DepositErc20Event::try_from(entry)?;
                    Ok(Event::DepositErc20(event))
                }
                _ => Err(ParseEventError::UnknownEventSignature(entry.topics[0].clone())),
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct DepositEthEvent {
        pub sender: H160,
        pub recipient: Principal,
        pub amount: U256,
    }

    impl DepositEthEvent {
        pub fn signature() -> &'static str {
            "DepositEth(address,bytes32,uint256)"
        }
        pub fn topic() -> String {
            let topic = hex::encode(keccak256(Self::signature().as_bytes()));
            format!("0x{}", topic)
        }
    }

    impl TryFrom<LogEntry> for DepositEthEvent {
        type Error = ParseEventError;

        fn try_from(entry: LogEntry) -> Result<DepositEthEvent, ParseEventError> {
            if entry.topics.len() != 2 {
                return Err(ParseEventError::InvalidTopics);
            }
            let sender: H160 = parse_address_from_topic(&entry.topics[1])?;
            let recipient: Principal = parse_principal_from_topic(&entry.topics[2])?;
            let amount = U256::from_str_radix(&entry.data, 16)
                .map_err(|_| ParseEventError::FailedToParseUint)?;
            Ok(DepositEthEvent { sender, recipient, amount })
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
            "DepositErc20(address,bytes32,address,uint256)"
        }
        pub fn topic() -> String {
            let topic = hex::encode(keccak256(Self::signature().as_bytes()));
            format!("0x{}", topic)
        }
    }

    impl TryFrom<LogEntry> for DepositErc20Event {
        type Error = ParseEventError;

        fn try_from(entry: LogEntry) -> Result<DepositErc20Event, Self::Error> {
            if entry.topics.len() != 4 {
                return Err(ParseEventError::InvalidTopics);
            }
            let sender: H160 = parse_address_from_topic(&entry.topics[1])?;
            let recipient: Principal = parse_principal_from_topic(&entry.topics[2])?;
            let token: H160 = parse_address_from_topic(&entry.topics[3])?;
            let amount = U256::from_str_radix(&entry.data, 16)
                .map_err(|_| ParseEventError::FailedToParseUint)?;
            Ok(DepositErc20Event { sender, recipient, token, amount })
        }
    }
}

pub fn parse_address_from_topic(topic: &str) -> Result<H160, ParseEventError> {
    if topic.len() != 66 {
        return Err(ParseEventError::InvalidTopics)
    }
    H160::from_str(&topic[26..])
        .map_err(|_| ParseEventError::FailedToParseAddress)
}