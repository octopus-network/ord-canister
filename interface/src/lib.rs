//! The data types for using https://github.com/octopus-network/ord-canister
//!
//! # Example
//!
//! ```
//! use rune_indexer_interface::*;
//!
//! let indexer = Principal::from_text("o25oi-jaaaa-aaaal-ajj6a-cai").unwrap();
//! let (result,): (Result<Vec<RuneBalance>, OrdError>,) = ic_cdk::call(indexer, "get_runes_by_utxo", ("ee8345590d85047c66a0e131153e5202b9bda3990bd07decd9df0a9bb2589348", 0)).await.unwrap();
//! ```

use candid::CandidType;
use thiserror::Error;

/// The RuneId is a unique identifier for a RUNE.
#[derive(Debug, Eq, PartialEq, Copy, Clone, CandidType)]
pub struct RuneId {
  pub block: u64,
  pub tx: u32,
}

/// The RuneId, Balance pair
#[derive(Copy, Eq, PartialEq, Clone, Debug, CandidType)]
pub struct RuneBalance {
  pub id: RuneId,
  pub balance: u128,
}

#[derive(Debug, Eq, PartialEq, Error, CandidType)]
pub enum OrdError {
  #[error("params: {0}")]
  Params(String),
  #[error("overflow")]
  Overflow,
  #[error("block verification")]
  BlockVerification(u32),
  #[error("index error: {0}")]
  Index(#[from] MintError),
  #[error("rpc error: {0}")]
  Rpc(#[from] RpcError),
}

#[derive(Debug, Clone, Error, Eq, PartialEq, CandidType)]
pub enum RpcError {
  #[error("IO error occured while calling {0} onto {1} due to {2}.")]
  Io(String, String, String),
  #[error("Decoding response of {0} from {1} failed due to {2}.")]
  Decode(String, String, String),
  #[error("Received an error of endpoint {0} from {1}: {2}.")]
  Endpoint(String, String, String),
}

#[derive(Debug, Clone, Error, Eq, PartialEq, CandidType)]
pub enum MintError {
  #[error("limited to {0} mints")]
  Cap(u128),
  #[error("mint ended on block {0}")]
  End(u64),
  #[error("mint starts on block {0}")]
  Start(u64),
  #[error("not mintable")]
  Unmintable,
}
