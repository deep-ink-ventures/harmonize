use std::collections::{BTreeMap, HashMap};
use candid::{types::principal, CandidType, Principal};
use ethers_core::types::{H160, U256};
use thiserror::Error;
use crate::{access_control::caller_has_access, chain_fusion::{eth_send_raw_transaction::transfer_eth, job::submit_result::transfer_erc20}, read_state, state::mutate_state, HarmonizeError};

#[derive(Error, Debug, CandidType)]
pub enum WalletError {
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Not found")]
    NotFound,
}

#[derive(Default)]
pub struct Wallets {
    pub wallets: HashMap<H160, Wallet>,
}

impl Wallets {
    pub fn get(&self, wallet: H160) -> Result<&Wallet, WalletError> {
        self.wallets.get(&wallet).ok_or_else(|| WalletError::NotFound)
    }

    pub fn get_mut(&mut self, wallet: H160) -> Result<&mut Wallet, WalletError> {
        self.wallets.get_mut(&wallet).ok_or_else(|| WalletError::NotFound)
    }

    pub fn get_or_create_mut(&mut self, wallet: H160) -> &mut Wallet {
        self.wallets.entry(wallet).or_insert(Wallet::new(wallet))
    }

    pub fn insert(&mut self, wallet: Wallet) {
        self.wallets.insert(wallet.address, wallet);
    }

    pub fn transfer(
        &mut self,
        network_id: u32,
        token: H160,
        from: H160,
        to: H160,
        amount: U256
    ) -> Result<(), WalletError> {
        ic_cdk::println!("Transfer {} {}/{} from {} to {}", amount, network_id, token, from, to);
        self.debit( from, network_id, token, amount)?;
        self.create_and_credit( to, network_id, token, amount)?;
        Ok(())
    }

    pub fn transfer_native(
        &mut self,
        network_id: u32,
        from: H160,
        to: H160,
        amount: U256
    ) -> Result<(), WalletError> {
        ic_cdk::println!("Transfer {} {}/native from {} to {}", amount, network_id, from, to);
        self.debit_native( from, network_id, amount)?;
        self.create_and_credit_native( to, network_id, amount)?;
        Ok(())
    }

    pub fn create_and_credit(&mut self, wallet: H160, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        self.get_or_create_mut(wallet).credit(network_id, token, amount)
    }

    pub fn create_and_credit_native(&mut self, wallet: H160, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        self.get_or_create_mut(wallet).credit_native(network_id, amount)
    }

    pub fn create_default(&mut self, wallet: H160) {
        self.wallets.entry(wallet).or_insert(Wallet::new(wallet));
    }

    pub fn credit(&mut self, wallet: H160, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        self.get_mut(wallet)?.credit(network_id, token, amount)
    }

    pub fn credit_native(&mut self, wallet: H160, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        self.get_mut(wallet)?.credit_native(network_id, amount)
    }

    pub fn debit(&mut self, wallet: H160, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        self.get_mut(wallet)?.debit(network_id, token, amount)
    }

    pub fn debit_native(&mut self, wallet: H160, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        self.get_mut(wallet)?.debit_native(network_id, amount)
    }

    pub fn get_balance(&self, wallet: H160, network_id: u32, token: H160) -> Option<U256> {
        self.wallets.get(&wallet).and_then(|wallet| {
            wallet.get_balance(network_id, token)
        })
    }

    pub fn get_native_balance(&self, wallet: H160, network_id: u32) -> Option<U256> {
        self.wallets.get(&wallet).and_then(|wallet| {
            wallet.get_native_balance(network_id)
        })
    }
}

pub struct Wallet {
    pub address: H160,
    pub native_currency_balances: BTreeMap<u32, U256>,
    pub erc20_token_balances: BTreeMap<H160, BTreeMap<u32, U256>>,
}

impl Wallet {

    pub fn new(address: H160) -> Self {
        Wallet {
            address,
            native_currency_balances: BTreeMap::new(),
            erc20_token_balances: BTreeMap::new(),
        }
    }

    /// Increases the balance of the given address by the given amount.
    /// Returns the new balance.
    pub fn credit(&mut self, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        let balances = self.erc20_token_balances.entry(token).or_insert_with(|| BTreeMap::new());
        let balance = balances.entry(network_id).or_insert(U256::zero());
        *balance = balance.checked_add(amount).ok_or_else(|| WalletError::ArithmeticOverflow)?;
        ic_cdk::println!("Credited {} with {} {}/{}", self.address, amount, network_id, token);
        Ok(*balance)
    }

    /// Increases the balance of the given address by the given amount.
    /// Returns the new balance.
    pub fn credit_native(&mut self, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        let balance = self.native_currency_balances.entry(network_id).or_insert(U256::zero());
        *balance = balance.checked_add(amount).ok_or_else(|| WalletError::ArithmeticOverflow)?;
        ic_cdk::println!("Credited {} with {} {}/native", self.address, amount, network_id);
        Ok(*balance)
    }

    /// Withdraws the given amount from the wallet.
    /// Returns the new balance.
    pub fn debit(&mut self, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        let balance = self.erc20_token_balances
            .get_mut(&token)
            .and_then(|balances| {
                balances.get_mut(&network_id)
            });
        if let Some(balance) = balance {
            if *balance >= amount {
                *balance -= amount;
                ic_cdk::println!("Debited {} with {} {}/{}", self.address, amount, network_id, token);
                return Ok(*balance)
            } else {
                return Err(WalletError::InsufficientBalance);
            }
        }
        Ok(U256::zero())
    }

    /// Withdraws the given amount from the wallet.
    /// Returns the new balance.
    pub fn debit_native(&mut self, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        let balance = self.native_currency_balances.get_mut(&network_id);
        if let Some(balance) = balance {
            if *balance >= amount {
                *balance -= amount;
                ic_cdk::println!("Debited {} with {} {}/native", self.address, amount, network_id);
                return Ok(*balance)
            } else {
                return Err(WalletError::InsufficientBalance);
            }
        }
        Ok(U256::zero())
    }

    fn get_balance(&self, network_id: u32, token: H160) -> Option<U256> {
        self.erc20_token_balances.get(&token).and_then(|balances| {
            balances.get(&network_id).cloned()
        })
    }

    fn get_native_balance(&self, network_id: u32) -> Option<U256> {
        self.native_currency_balances.get(&network_id).cloned()
    }
}

// Public API

pub fn get_balance(wallet: H160, network_id: u32, token: H160) -> U256 {
    read_state(|s| {
        s.wallets.get_balance(wallet, network_id, token)
            .unwrap_or(U256::zero())
    })
}

pub fn get_native_balance(wallet: H160, network_id: u32) -> U256 {
    read_state(|s| {
        s.wallets.get_native_balance(wallet, network_id)
            .unwrap_or(U256::zero())
    })
}

pub fn transfer(from: H160, to: H160, network_id: u32, token: H160, amount: U256) -> Result<(), HarmonizeError> {
    caller_has_access(from)?;
    mutate_state(|s| {
        s.wallets.transfer(network_id, token, from, to, amount)
    })?;
    Ok(())
}

pub fn transfer_native(from: H160, to: H160, network_id: u32, amount: U256) -> Result<(), HarmonizeError> {
    caller_has_access(from)?;
    mutate_state(|s| {
        s.wallets.transfer_native(network_id, from, to, amount)
    })?;
    Ok(())
}

pub async fn withdraw(from: H160, network_id: u32, token: H160, amount: U256) -> Result<(), HarmonizeError> {
    caller_has_access(from)?;
    mutate_state(|s| {
        s.wallets.debit(from, network_id, token, amount)
    })?;
    transfer_erc20(network_id, token, from, amount).await;
    Ok(())
}

pub async fn withdraw_native(from: H160, network_id: u32, amount: U256) -> Result<(), HarmonizeError> {
    caller_has_access(from)?;
    mutate_state(|s| {
        s.wallets.debit_native(from, network_id, amount)
    })?;
    transfer_eth(network_id, amount, from, None).await;
    Ok(())
}

pub struct EvmAccount {
    pub address: H160,
    pub native: BTreeMap<u32, U256>,
    pub erc20: BTreeMap<u32, BTreeMap<H160, U256>>,
}

impl EvmAccount {
    pub fn new(address: H160) -> Self {
        Self {
            address,
            native: BTreeMap::new(),
            erc20: BTreeMap::new(),
        }
    }

    /// Increases the balance of the given address by the given amount.
    /// Returns the new balance.
    pub fn credit_native(&mut self, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        let balance = self.native.entry(network_id).or_insert(U256::zero());
        *balance = balance.checked_add(amount).ok_or_else(|| WalletError::ArithmeticOverflow)?;
        ic_cdk::println!("Credited {} with {} {}/native", self.address, amount, network_id);
        Ok(*balance)
    }

    /// Withdraws the given amount from the wallet.
    /// Returns the new balance.
    pub fn debit_native(&mut self, network_id: u32, amount: U256) -> Result<U256, WalletError> {
        let balance = self.native.get_mut(&network_id);
        if let Some(balance) = balance {
            if *balance >= amount {
                *balance -= amount;
                ic_cdk::println!("Debited {} with {} {}/native", self.address, amount, network_id);
                return Ok(*balance)
            } else {
                return Err(WalletError::InsufficientBalance);
            }
        }
        Ok(U256::zero())
    }

    /// Increases the balance of the given address by the given amount.
    /// Returns the new balance.
    pub fn credit_erc20(&mut self, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        let balances = self.erc20.entry(network_id).or_insert_with(|| BTreeMap::new());
        let balance = balances.entry(token).or_insert(U256::zero());
        *balance = balance.checked_add(amount).ok_or_else(|| WalletError::ArithmeticOverflow)?;
        ic_cdk::println!("Credited {} with {} {}/{}", self.address, amount, network_id, token);
        Ok(*balance)
    }

    /// Withdraws the given amount from the wallet.
    /// Returns the new balance.
    pub fn debit_erc20(&mut self, network_id: u32, token: H160, amount: U256) -> Result<U256, WalletError> {
        let balance = self.erc20
            .get_mut(&network_id)
            .and_then(|balances| {
                balances.get_mut(&token)
            });
        if let Some(balance) = balance {
            if *balance >= amount {
                *balance -= amount;
                ic_cdk::println!("Debited {} with {} {}/{}", self.address, amount, network_id, token);
                return Ok(*balance)
            } else {
                return Err(WalletError::InsufficientBalance);
            }
        }
        Ok(U256::zero())
    }

    pub fn get_balance(&self, network_id: u32, token: H160) -> Option<U256> {
        self.erc20.get(&network_id).and_then(|balances| {
            balances.get(&token).cloned()
        })
    }

    pub fn get_native_balance(&self, network_id: u32) -> Option<U256> {
        self.native.get(&network_id).cloned()
    }
}