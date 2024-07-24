pub mod chain_fusion;
pub mod signer;
pub mod access_control;
pub mod wallet;
pub mod types;
pub mod state;
pub mod declarations;

use candid::{CandidType, Nat, Principal};
use thiserror::Error;
use wallet::WalletError;
use access_control::{AccessControlError, SignInError};
use state::{read_state, Init, NetworkMut};
use types::{H160t, U256t};

#[derive(Error, Debug, CandidType)]
pub enum HarmonizeError {
    #[error("Access control: {0}")]
    AccessControlError(#[from] AccessControlError),
    #[error("Wallet: {0}")]
    WalletError(#[from] WalletError),
    #[error("Sign in: {0}")]
    SignInError(#[from] SignInError)
}

#[ic_cdk::init]
fn init(args: Init) {
    println!("Initialized canister with: {:?}", args);
    state::initialize_state(args.into());
    chain_fusion::setup_timers();
}

/*
 * Canister Settings
 */

#[ic_cdk::update]
fn set_owner(new_owner: Principal) {
    state::set_owner(new_owner);
}

#[ic_cdk::query]
fn get_owner() -> Principal {
    state::get_owner()
}

#[ic_cdk::update]
fn set_network_config(chain_id: u32, network_mut: NetworkMut) {
    state::set_network_config(chain_id, network_mut);
}

#[ic_cdk::query]
fn get_deposit_address() -> H160t {
    state::get_deposit_address().into()
}

#[ic_cdk::query]
fn get_last_processed_block(chain_id: u32) -> Nat {
    state::get_last_processed_block(chain_id)
}

/*
 * Access Control
 */

#[ic_cdk::query]
async fn has_access(principal: Principal, wallet: H160t) -> bool {
    access_control::has_access(principal, wallet.into()).is_ok()
}

#[ic_cdk::update]
async fn sign_in_challenge(wallet: H160t) -> Result<String, HarmonizeError> {
    Ok(access_control::sign_in_challenge(wallet.into()).await?)
}

#[ic_cdk::update]
async fn sign_in_with_signature(wallet: H160t, signature: String) -> Result<bool, HarmonizeError> {
    Ok(access_control::sign_in_with_signature(wallet.into(), signature).await?)
}

#[ic_cdk::update]
async fn sign_in_with_ethereum() -> Result<(), HarmonizeError> {
    Ok(access_control::sign_in_with_ethereum().await?)
}

#[ic_cdk::query]
async fn get_siwe_session_address() -> Result<H160t, HarmonizeError> {
    Ok(access_control::get_siwe_session_address().await?.into())
}

/*
 * Virtual Accounts
 */

#[ic_cdk::query]
fn get_erc20_balance(wallet: Principal, network_id: u32, token: H160t) -> U256t {
    wallet::get_erc20_balance(wallet, network_id, token.into()).into()
}

#[ic_cdk::query]
fn get_eth_balance(wallet: Principal, network_id: u32) -> U256t {
    wallet::get_eth_balance(wallet, network_id).into()
}

#[ic_cdk::update]
fn transfer_erc20(from: Principal, to: Principal, network_id: u32, token: H160t, amount: U256t) -> Result<(), HarmonizeError> {
    wallet::transfer_erc20(from, to, network_id, token.into(), amount.into())
}

#[ic_cdk::update]
fn transfer_eth(from: Principal, to: Principal, network_id: u32, amount: U256t) -> Result<(), HarmonizeError> {
    wallet::transfer_eth(from, to, network_id, amount.into())
}

#[ic_cdk::update]
async fn withdraw_erc20(to: H160t, network_id: u32, token: H160t, amount: U256t) -> Result<(), HarmonizeError> {
    wallet::withdraw_erc20(ic_cdk::caller(), to.into(), network_id, token.into(), amount.into()).await
}

#[ic_cdk::update]
async fn withdraw_eth(to: H160t, network_id: u32, amount: U256t) -> Result<(), HarmonizeError> {
    wallet::withdraw_eth(ic_cdk::caller(), to.into(), network_id, amount.into()).await
}

// Enable Candid export
ic_cdk::export_candid!();
