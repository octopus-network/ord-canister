mod bitcoin_api;
pub mod config;
pub mod ic_log;
pub mod index;
mod into_usize;
mod notifier;
pub mod rpc;

use anyhow::{anyhow, bail, ensure, Context, Error};
pub use bitcoin::{
  address::{Address, NetworkUnchecked},
  block::Header,
  blockdata::{
    constants::{DIFFCHANGE_INTERVAL, MAX_SCRIPT_ELEMENT_SIZE, SUBSIDY_HALVING_INTERVAL},
    locktime::absolute::LockTime,
  },
  consensus::{self, encode, Decodable, Encodable},
  hash_types::{BlockHash, TxMerkleNode},
  hashes::Hash,
  script, Amount, Block, Network, OutPoint, Script, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
  Txid, Witness,
};
use chrono::{DateTime, TimeZone, Utc};
use core2::io::Cursor;
pub use index::entry::RuneEntry;
use into_usize::IntoUsize;
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use runes_indexer_interface::OrdError;
use serde::{Deserialize, Deserializer, Serialize};
use std::cell::RefCell;

type Result<T = (), E = Error> = std::result::Result<T, E>;

pub fn timestamp(seconds: u64) -> DateTime<Utc> {
  Utc
    .timestamp_opt(seconds.try_into().unwrap_or(i64::MAX), 0)
    .unwrap()
}
