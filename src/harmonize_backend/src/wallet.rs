use std::{collections::HashMap, fmt::Debug, hash::Hash};
use candid::{CandidType, Principal};
use ethers_core::types::{H160, U256};
use thiserror::Error;
use typemap::TypeMap;
use unsafe_any::UnsafeAny;
use crate::{chain_fusion::job::safe, read_state, state::mutate_state, HarmonizeError};

pub mod balances {
    use std::{collections::BTreeMap, fmt::{Debug, Display}, ops::{Sub, SubAssign}};
    use candid::CandidType;
    use ethers_core::types::{H160, U256};
    use thiserror::Error;
    
    

    pub trait CheckedAdd: Sized {
        fn checked_add(self, other: Self) -> Option<Self>;
    }

    impl CheckedAdd for U256 {
        fn checked_add(self, other: Self) -> Option<Self> {
            U256::checked_add(self, other)
        }
    }

    pub trait Zero: Sized {
        fn zero() -> Self;
    }

    impl Zero for U256 {
        fn zero() -> Self {
            U256::zero()
        }
    }

    #[derive(Error, Debug, CandidType)]
    pub enum BalanceError {
        #[error("Arithmetic overflow")]
        ArithmeticOverflow,
        #[error("Insufficient balance")]
        InsufficientBalance,
        #[error("Not found")]
        NotFound,
    }

    fn safe_transfer<Value>(from: &mut Value, to: &mut Value, amount: Value) -> Result<(), BalanceError>
    where 
        Value: Clone + CheckedAdd + Ord + SubAssign + Zero
    {
        if *from < amount {
            return Err(BalanceError::InsufficientBalance);
        }
        *to = to.clone().checked_add(amount.clone()).ok_or(BalanceError::ArithmeticOverflow)?;
        *from -= amount;
        Ok(())
    }

    pub trait BalanceStore {
        type Key;
        type Value: Clone + Ord + Zero + CheckedAdd + SubAssign;

        fn credit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError>;
        fn debit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError>;
        fn get_or_create_mut(&mut self, key: &Self::Key) -> &mut Self::Value;
        fn get_or_create(&mut self, key: &Self::Key) -> &Self::Value {
            self.get_or_create_mut(key)
        }
        fn get(&self, key: &Self::Key) -> Option<&Self::Value>;
        fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value>;

        fn transfer(&mut self, to: &mut Self, key: &Self::Key, amount: Self::Value) -> Result<(), BalanceError> {
            assert!(amount >= Self::Value::zero(), "Amount must be positive");
            let from = self.get_mut(key).ok_or(BalanceError::NotFound)?;
            let to = to.get_or_create_mut(key);
            safe_transfer(from, to, amount)
        }
    }

    pub struct Balances<Key, Value> (BTreeMap<Key, Value>);

    impl<Key, Value> Balances<Key, Value> {
        pub fn new() -> Self {
            Balances(BTreeMap::new())
        }
        fn keys(&self) -> impl Iterator<Item=&Key> {
            self.0.keys()
        }
    }

    impl<Key, Value> BalanceStore for Balances<Key, Value>
    where 
        Key: Clone + Display + Ord,
        Value: Clone + CheckedAdd + Ord + SubAssign + Zero + Sub<Output=Value>
    {
        type Key = Key;
        type Value = Value;

        fn credit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError> {
            assert!(amount >= Value::zero(), "Amount must be positive");

            let balance = self.0.entry(key.clone()).or_insert_with(Zero::zero);
            *balance = balance.clone().checked_add(amount).ok_or(BalanceError::ArithmeticOverflow)?;
            Ok(balance.clone())
        }

        fn debit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError> {
            assert!(amount >= Zero::zero(), "Amount must be positive");

            let balance = self.0
                .get_mut(key)
                .ok_or(BalanceError::InsufficientBalance)
                .and_then(|balance| {
                    if *balance >= amount {
                        *balance -= amount;
                        Ok(balance.clone())
                    } else {
                        Err(BalanceError::InsufficientBalance)
                    }
                });

            // Remove the balance if it is zero
            match &balance {
                Ok(b) if b == &Zero::zero() => {
                    self.0.remove(key);
                }
                _ => {}
            }
            balance
        }

        fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
            self.0.get(key)
        }

        fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value> {
            self.0.get_mut(key)
        }
        
        fn get_or_create_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
            self.0.entry(key.clone()).or_insert_with(Zero::zero)
        }
    }

    impl<Key, Value> Default for Balances<Key, Value> {
        fn default() -> Self {
            Balances::new()
        }
    }

    pub struct GroupedBalances<Group, Key, Value> (BTreeMap<Group, Balances<Key, Value>>);

    impl<Group, Key, Value> GroupedBalances<Group, Key, Value>
    where 
        Group: Ord
    {
        pub fn new() -> Self {
            GroupedBalances(BTreeMap::new())
        }
        pub fn groups(&self) -> impl Iterator<Item=&Group> {
            self.0.keys()
        }
        pub fn group_keys(&self, group: &Group) -> Option<impl Iterator<Item=&Key>> {
            self.0.get(group).map(|balances| balances.keys())
        }
    }

    impl<Group, Key, Value> BalanceStore for GroupedBalances<Group, Key, Value>
    where 
        Group: Clone + Display + Ord,
        Key: Clone + Display + Ord,
        Value: Clone + CheckedAdd + Ord + SubAssign + Zero + Sub<Output=Value>
    {
        type Key = (Group, Key);
        type Value = Value;
        
        fn credit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError> {
            let (group, key) = key;
            self.0.entry(group.clone()).or_default().credit(key, amount)
        }
        
        fn debit(&mut self, key: &Self::Key, amount: Self::Value) -> Result<Self::Value, BalanceError> {
            let (group, key) = key;
            match self.0.get_mut(group).ok_or(BalanceError::NotFound)?.debit(key, amount) {
                Ok(balance) if balance == Zero::zero() => {
                    if self.0.get(group).map(|balances| balances.0.is_empty()).unwrap_or(false) {
                        self.0.remove(group);
                    }
                    Ok(balance)
                },
                result => result
            }
        }
        
        fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
            let (group, key) = key;
            self.0.get(group)?.get(key)
        }
        
        fn get_mut(&mut self, key: &Self::Key) -> Option<&mut Self::Value> {
            let (group, key) = key;
            self.0.get_mut(group)?.get_mut(key)
        }

        fn get_or_create_mut(&mut self, key: &Self::Key) -> &mut Self::Value {
            let (group, key) = key;
            self.0.entry(group.clone()).or_default().get_or_create_mut(key)
        }
    }

    impl Default for GroupedBalances<u32, H160, U256> {
        fn default() -> Self {
            GroupedBalances::new()
        }
    }
}

#[derive(Error, Debug, CandidType)]
pub enum WalletError {
    #[error(transparent)]
    BalanceError(#[from] BalanceError),
    #[error("Wallet not found")]
    NotFound
}

#[derive(Default)]
pub struct Wallets<Id> {
    pub wallets: HashMap<Id, Wallet<Id>>,
}

impl<Id> Wallets<Id>
where 
    Id: Eq + Hash + Clone
{
    pub fn new() -> Self {
        Wallets {
            wallets: HashMap::default(),
        }
    }
    pub fn exists(&self, id: Id) -> bool {
        self.wallets.contains_key(&id)
    }

    pub fn get(&self, wallet: Id) -> Option<&Wallet<Id>> {
        self.wallets.get(&wallet)
    }

    pub fn get_mut(&mut self, wallet: Id) -> Option<&mut Wallet<Id>> {
        self.wallets.get_mut(&wallet)
    }

    pub fn get_or_create_mut(&mut self, wallet: Id) -> &mut Wallet<Id> {
        self.wallets.entry(wallet.clone()).or_insert(Wallet::new(wallet))
    }

    pub fn insert(&mut self, wallet: Wallet<Id>) {
        self.wallets.insert(wallet.id.clone(), wallet);
    }

    pub fn transfer<K>(
        &mut self,
        from: Id,
        to: Id,
        key: &<K::Value as BalanceStore>::Key,
        amount: <K::Value as BalanceStore>::Value
    ) -> Result<(), WalletError>
    where 
        K: Key,
        K::Value: BalanceStore + Default
    {
        self.debit::<K>(from.clone(), key, amount.clone())?;
        if let Err(e) = self.credit::<K>(to, key, amount.clone()) {
            self.credit::<K>(from, key, amount).expect("BUG: Failed to rollback");
            return Err(e);
        }
        Ok(())
    }

    pub fn credit<K>(
        &mut self,
        wallet: Id,
        key: &<K::Value as BalanceStore>::Key,
        amount: <K::Value as BalanceStore>::Value
    ) -> Result<(), WalletError>
    where 
        K: Key,
        K::Value: BalanceStore + Default
    {
        self.get_or_create_mut(wallet).credit::<K>(key, amount)?;
        Ok(())
    }

    pub fn create_default(&mut self, wallet: Id) {
        self.wallets.entry(wallet.clone()).or_insert(Wallet::new(wallet));
    }

    pub fn debit<K>(
        &mut self,
        wallet: Id,
        key: &<K::Value as BalanceStore>::Key,
        amount: <K::Value as BalanceStore>::Value
    ) -> Result<(), WalletError>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.get_mut(wallet).ok_or(WalletError::NotFound)?.debit::<K>(key, amount)?;
        Ok(())
    }

    pub fn get_balance<K>(
        &self,
        wallet: Id,
        key: &<K::Value as BalanceStore>::Key,
    ) -> Option<&<K::Value as BalanceStore>::Value>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.get(wallet)?.get_balance::<K>(key)
    }

    pub fn get_balance_or_default<K>(
        &self,
        wallet: Id,
        key: &<K::Value as BalanceStore>::Key,
    ) -> <K::Value as BalanceStore>::Value
    where 
        K: Key,
        K::Value: BalanceStore,
        <K::Value as BalanceStore>::Value: Zero
    {
        self.get(wallet).and_then(|wallet| {
            wallet.get_balance::<K>(key)
        }).cloned().unwrap_or_else(Zero::zero)
    }
}

use balances::*;

pub struct Eth;
pub struct Erc20;

use typemap::Key;

impl Key for Eth {
    type Value = Balances<u32, U256>;
}

impl Key for Erc20 {
    type Value = GroupedBalances<u32, H160, U256>;
}

pub struct Wallet<Id> {
    pub id: Id,
    pub balances: TypeMap<dyn UnsafeAny>,
}

impl<Id> Wallet<Id> {
    pub fn new(id: Id) -> Self {
        Wallet {
            id,
            balances: TypeMap::new(),
        }
    }
    pub fn credit<K>(&mut self, key: &<K::Value as BalanceStore>::Key, amount: <K::Value as BalanceStore>::Value) -> Result<<K::Value as BalanceStore>::Value, WalletError>
    where 
        K: Key,
        K::Value: BalanceStore + Default
    {
        self.balances.entry::<K>().or_insert_with(Default::default).credit(key, amount).map_err(Into::into)
    }
    pub fn debit<K>(&mut self, key: &<K::Value as BalanceStore>::Key, amount: <K::Value as BalanceStore>::Value) -> Result<<K::Value as BalanceStore>::Value, WalletError>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.balances.get_mut::<K>().ok_or(WalletError::NotFound)?.debit(key, amount).map_err(Into::into)
    }
    pub fn get<K>(&self) -> Option<&K::Value>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.balances.get::<K>()
    }
    pub fn get_mut<K>(&mut self) -> Option<&mut K::Value>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.balances.get_mut::<K>()
    }
    pub fn get_balance<K>(&self, key: &<K::Value as BalanceStore>::Key) -> Option<&<K::Value as BalanceStore>::Value>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.balances.get::<K>().and_then(|balances| {
            balances.get(key)
        })
    }
    pub fn get_balance_mut<K>(&mut self, key: &<K::Value as BalanceStore>::Key) -> Option<&mut <K::Value as BalanceStore>::Value>
    where 
        K: Key,
        K::Value: BalanceStore
    {
        self.balances.get_mut::<K>().and_then(|balances| {
            balances.get_mut(key)
        })
    }
}

// Public API

pub fn get_erc20_balance(wallet: Principal, network_id: u32, token: H160) -> U256 {
    read_state(|s| {
        s.wallets.get_balance_or_default::<Erc20>(wallet, &(network_id, token))
    })
}

pub fn get_eth_balance(wallet: Principal, network_id: u32) -> U256 {
    read_state(|s| {
        s.wallets.get_balance_or_default::<Eth>(wallet, &network_id)
    })
}

pub fn transfer_erc20(from: Principal, to: Principal, network_id: u32, token: H160, amount: U256) -> Result<(), HarmonizeError> {
    mutate_state(|s| {
        s.wallets.transfer::<Erc20>(from, to, &(network_id, token), amount)
    })?;
    Ok(())
}

pub fn transfer_eth(from: Principal, to: Principal, network_id: u32, amount: U256) -> Result<(), HarmonizeError> {
    mutate_state(|s| {
        s.wallets.transfer::<Eth>(from, to, &network_id, amount)
    })?;
    Ok(())
}

pub async fn withdraw_erc20(from: Principal, to: H160, network_id: u32, token: H160, amount: U256) -> Result<(), HarmonizeError> {
    mutate_state(|s| {
        s.wallets.debit::<Erc20>(from, &(network_id, token), amount)
    })?;
    let caller = ic_cdk::caller();
    let result = safe::transfer_erc20(network_id, token, caller, to, amount, None, None).await;
    if let Err(e) = result {
        mutate_state(|s| {
            s.wallets.credit::<Erc20>(from, &(network_id, token), amount)
        })?;
        return Err(e.into());
    }
    Ok(())
}

pub async fn withdraw_eth(from: Principal, to: H160, network_id: u32, amount: U256) -> Result<(), HarmonizeError> {
    mutate_state(|s| {
        s.wallets.debit::<Eth>(from, &network_id, amount)
    })?;
    let caller = ic_cdk::caller();
    let result = safe::transfer_eth(network_id, caller, to, amount, None, None).await;
    if let Err(e) = result {
        mutate_state(|s| {
            s.wallets.credit::<Eth>(from, &network_id, amount)
        })?;
        return Err(e.into());
    }
    Ok(())
}