use std::{
    cmp::{min, Ordering},
    ops::{Add, Div, Sub},
    time::Duration,
};

use candid::Nat;
use ic_cdk::println;

use crate::{chain_fusion::{
    evm_rpc::{
        BlockTag, GetBlockByNumberResult, GetLogsArgs, GetLogsResult, HttpOutcallError,
        MultiGetBlockByNumberResult, MultiGetLogsResult, RejectionCode, RpcError, EVM_RPC,
    }, guard::TimerGuard, job::handle_event, TaskType
}, state::Network, types::H160Ext};
use crate::state::{read_state, read_network_state, mutate_network_state};

use super::evm_rpc::LogEntry;

async fn process_logs(network_id: u32) {
    // TODO: Move guard up one level
    let _guard = match TimerGuard::new(TaskType::ProcessLogs) {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let logs_to_process = read_network_state(network_id, |s| (s.logs_to_process.clone()));
    for (event_source, event) in logs_to_process {
        handle_event(network_id, event_source, event).await
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GetLogsError {
    #[error("Inconsistent results")]
    InconsistentResults,
    #[error("HTTP outcall error")]
    HttpOutcallError(HttpOutcallError),
    #[error("Canister call rejected: {0}")]
    CallRejected(String),
    #[error("RPC error")]
    RpcError(RpcError),
}

pub async fn get_logs(network_id: u32, from: &Nat, to: &Nat) -> Result<Vec<LogEntry>, GetLogsError> {
    let get_logs_address = read_network_state(network_id, |s| s.get_logs_address.clone());
    // let get_logs_topics = read_state(|s| s.get_logs_topics.clone());
    let rpc_services = read_network_state(network_id, |s| s.rpc_services.clone());

    if get_logs_address.is_empty() {
        println!("No addresses to query logs for");
        return Ok(vec![]);
    }

    // if get_logs_topics.is_none() {
    //     println!("No topics to query logs for");
    //     return Ok(vec![]);
    // }

    let get_logs_topics = None;

    let get_logs_args: GetLogsArgs = GetLogsArgs {
        fromBlock: Some(BlockTag::Number(from.clone())),
        toBlock: Some(BlockTag::Number(to.clone())),
        addresses: get_logs_address.into_iter().map(|a| a.to_repr()).collect(),
        topics: get_logs_topics,
    };

    let cycles = 10_000_000_000;
    let (result,) = EVM_RPC
        .eth_get_logs(rpc_services, None, get_logs_args, cycles)
        .await
        .map_err(|e| GetLogsError::CallRejected(e.1))?;

    match result {
        MultiGetLogsResult::Consistent(r) => match r {
            GetLogsResult::Ok(logs) => Ok(logs),
            GetLogsResult::Err(e) => {
                println!("Failed to get logs: {e:?}");
                Err(GetLogsError::RpcError(e))
            }
        },
        MultiGetLogsResult::Inconsistent(_) => Err(GetLogsError::InconsistentResults),
    }
}

/// Scraps Ethereum logs between `from` and `min(from + MAX_BLOCK_SPREAD, to)` since certain RPC providers
/// require that the number of blocks queried is no greater than MAX_BLOCK_SPREAD.
/// Returns the last block number that was scraped (which is `min(from + MAX_BLOCK_SPREAD, to)`) if there
/// was no error when querying the providers, otherwise returns `None`.
async fn scrape_eth_logs_range_inclusive(network_id: u32, from: &Nat, to: &Nat) -> Option<Nat> {
    /// The maximum block spread is introduced by Alchemy limits.
    /// TODO: Make this configurable.
    const MAX_BLOCK_SPREAD: u16 = 500;
    match from.cmp(to) {
        Ordering::Less | Ordering::Equal => {
            let max_to = from.clone().add(Nat::from(MAX_BLOCK_SPREAD));
            let mut last_block_number = min(max_to, to.clone());
            println!(
                "Scraping ETH logs from block {:?} to block {:?}...",
                from, last_block_number
            );

            let logs = loop {
                match get_logs(network_id, from, &last_block_number).await {
                    Ok(logs) => break logs,
                    Err(e) => {
                        println!(
                          "Failed to get ETH logs from block {from} to block {last_block_number}: {e:?}",
                        );
                        match e {
                            GetLogsError::RpcError(RpcError::HttpOutcallError(e)) => {
                                if e.is_response_too_large() {
                                    if *from == last_block_number {
                                        mutate_network_state(network_id, |s| {
                                            s.record_skipped_block(last_block_number.clone());
                                            s.last_scraped_block_number = last_block_number.clone();
                                        });
                                        return Some(last_block_number);
                                    } else {
                                        let new_last_block_number = from.clone().add(
                                            last_block_number
                                                .clone()
                                                .sub(from.clone())
                                                .div(Nat::from(2u32)),
                                        );
                                        println!( "Too many logs received in range [{from}, {last_block_number}]. Will retry with range [{from}, {new_last_block_number}]");
                                        last_block_number = new_last_block_number;
                                        continue;
                                    }
                                }
                            }
                            _ => return None,
                        }
                    }
                };
            };

            for log_entry in logs {
                println!("Received event {log_entry:?}",);
                mutate_network_state(network_id, |s| s.record_log_to_process(&log_entry));
            }

            if read_network_state(network_id, Network::has_logs_to_process) {
                println!("Found logs to process",);
                let last_block_number_clone = last_block_number.clone();

                // Process logs in a separate task to avoid blocking the current task
                ic_cdk_timers::set_timer(Duration::from_secs(0), move || {
                    ic_cdk::spawn(async move {
                        process_logs(network_id).await;
                        mutate_network_state(network_id, |s| {
                            let n = s.last_processed_block_number.clone().unwrap_or(Nat::from(0u32));
                            if n < last_block_number_clone {
                                s.last_processed_block_number = Some(last_block_number_clone)
                            }
                        });
                    })
                });
            }

            mutate_network_state(network_id, |s| s.last_scraped_block_number = last_block_number.clone());
            Some(last_block_number)
        }
        // TODO: Don't trap, return an error instead
        Ordering::Greater => {
            ic_cdk::trap(&format!(
              "BUG: last scraped block number ({:?}) is greater than the last queried block number ({:?})",
              from, to
          ));
        }
    }
}

pub async fn scrape_eth_logs_on_all_networks() {
    let network_ids = read_state(|s| s.networks.keys().cloned().collect::<Vec<u32>>());
    for network_id in network_ids {
        scrape_eth_logs(network_id).await;
    }
}

pub async fn scrape_eth_logs(network_id: u32) {
    let _guard = match TimerGuard::new(TaskType::ScrapeLogs) {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let last_block_number = match update_last_observed_block_number(network_id).await {
        Ok(Some(block_number)) => block_number,
        Ok(None) => {
            println!(
                "[scrape_eth_logs]: skipping scraping ETH logs: no last observed block number"
            );
            return;
        }
        Err(e) => {
            println!(
                "[scrape_eth_logs]: skipping scraping ETH logs: failed to get the last observed block number: {e:?}"
            );
            return;
        }
    };

    let mut last_scraped_block_number = read_network_state(network_id, |s| s.last_scraped_block_number.clone());

    while last_scraped_block_number < last_block_number {
        let next_block_to_query = last_scraped_block_number.add(Nat::from(1u32));
        last_scraped_block_number =
            match scrape_eth_logs_range_inclusive(network_id, &next_block_to_query, &last_block_number).await {
                Some(last_scraped_block_number) => last_scraped_block_number,
                None => {
                    return;
                }
            };
    }
}

pub enum GetBlockNumberError {
    InconsistentResults,
}

async fn update_last_observed_block_number(network_id: u32) -> Result<Option<Nat>, GetLogsError> {
    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let block_tag = read_network_state(network_id, |s| s.block_tag.clone());

    let cycles = 10_000_000_000;
    let (result,) = EVM_RPC
        .eth_get_block_by_number(rpc_providers, None, block_tag, cycles)
        .await
        .map_err(|e| GetLogsError::CallRejected(e.1))?;

    match result {
        MultiGetBlockByNumberResult::Consistent(r) => match r {
            GetBlockByNumberResult::Ok(latest_block) => {
                let block_number = Some(latest_block.number);
                mutate_network_state(network_id, |s| s.last_observed_block_number.clone_from(&block_number));
                Ok(block_number)
            }
            GetBlockByNumberResult::Err(err) => {
                println!("Failed to get the latest finalized block number: {err:?}");
                Ok(read_network_state(network_id, |s| s.last_observed_block_number.clone()))
            }
        },
        MultiGetBlockByNumberResult::Inconsistent(_) => {
            Err(GetLogsError::InconsistentResults)
        }
    }
}

impl HttpOutcallError {
    pub fn is_response_too_large(&self) -> bool {
        match self {
            Self::IcError { code, message } => is_response_too_large(code, message),
            _ => false,
        }
    }
}

pub fn is_response_too_large(code: &RejectionCode, message: &str) -> bool {
    code == &RejectionCode::SysFatal && message.contains("size limit")
}
