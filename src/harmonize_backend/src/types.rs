use candid::{CandidType, Nat};
use ethers_core::{abi::ethereum_types::FromStrRadixErr, types::{H160, U256}};
use serde::{Serialize, Deserialize};
use std::{rc::Rc, str::FromStr};


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct H160t (pub H160);

impl Into<H160> for H160t {
    fn into(self) -> H160 {
        self.0
    }
}

impl From<H160> for H160t {
    fn from(h: H160) -> Self {
        H160t(h)
    }
}

impl Serialize for H160t {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.to_repr().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for H160t {
    fn deserialize<D>(deserializer: D) -> Result<H160t, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        H160::from_str(&s).map_err(serde::de::Error::custom).map(H160t)
    }
}

impl FromStr for H160t {
    type Err = <H160 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        H160::from_str(s).map(H160t)
    }
}

impl CandidType for H160t {
    fn _ty() -> candid::types::Type {
        candid::types::Type(Rc::new(candid::types::TypeInner::Text))
    }

    fn idl_serialize<S: candid::types::Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_text(&self.to_string())
    }
}

impl std::fmt::Display for H160t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_repr())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct U256t (pub U256);

impl Into<U256> for U256t {
    fn into(self) -> U256 {
        self.0
    }
}

impl From<U256> for U256t {
    fn from(h: U256) -> Self {
        U256t(h)
    }
}

impl Serialize for U256t {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{}", self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for U256t {
    fn deserialize<D>(deserializer: D) -> Result<U256t, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        U256::from_dec_str(&s).map_err(serde::de::Error::custom).map(U256t)
    }
}

impl FromStr for U256t {
    type Err = FromStrRadixErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        U256::from_str_radix(s, 10).map(U256t)
    }
}

impl CandidType for U256t {
    fn _ty() -> candid::types::Type {
        candid::types::Type(Rc::new(candid::types::TypeInner::Text))
    }

    fn idl_serialize<S: candid::types::Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_text(&self.to_string())
    }
}

impl std::fmt::Display for U256t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

pub trait H160Ext {
    fn to_repr(&self) -> String;
}

impl H160Ext for H160 {
    fn to_repr(&self) -> String {
        format!("0x{:x}", self)
    }
}

pub trait H256Ext {
    fn to_repr(&self) -> String;
}

impl H256Ext for U256 {
    fn to_repr(&self) -> String {
        format!("0x{:x}", self)
    }
}

pub trait NatExt {
    fn to_u256(&self) -> U256;
}

impl NatExt for Nat {
    fn to_u256(&self) -> U256 {
        let be_bytes = self.0.to_bytes_be();
        U256::from_big_endian(&be_bytes)
    }
}