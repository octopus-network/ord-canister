use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::management_canister::bitcoin::BitcoinNetwork;
use ic_stable_structures::storable::{Bound, Storable};
use serde::Serialize;
use std::borrow::Cow;

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Config {
  pub network: BitcoinNetwork,
  pub bitcoin_rpc_url: String,
  pub subscribers: Vec<Principal>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      network: BitcoinNetwork::Regtest,
      bitcoin_rpc_url: "".to_string(),
      subscribers: vec![],
    }
  }
}

impl Config {
  pub fn get_subnet_nodes(&self) -> u64 {
    match self.network {
      BitcoinNetwork::Regtest => 13,
      BitcoinNetwork::Testnet => 13,
      BitcoinNetwork::Mainnet => 34,
    }
  }
}

impl Storable for Config {
  fn to_bytes(&self) -> Cow<[u8]> {
    let bytes = bincode::serialize(self).unwrap();
    Cow::Owned(bytes)
  }

  fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
    bincode::deserialize(bytes.as_ref()).unwrap()
  }

  const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct UpgradeArgs {
  pub bitcoin_rpc_url: Option<String>,
  pub subscribers: Option<Vec<Principal>>,
}

#[derive(CandidType, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum RunesIndexerArgs {
  Init(Config),
  Upgrade(Option<UpgradeArgs>),
}
