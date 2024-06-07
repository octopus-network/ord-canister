mod index;
mod rpc;
mod runes;

use self::index::entry::{OutPointValue, TxidValue};
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
use ic_stable_memory::collections::{SBTreeMap, SHashMap, SVec};
use index::entry::{RuneBalance, RuneEntry};
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use std::cell::RefCell;

pub(crate) type Result<T, E> = std::result::Result<T, OrdError>;

#[derive(Debug, Error, CandidType)]
pub enum OrdError {
  #[error("overflow")]
  Overflow,
  #[error("index error: {0}")]
  Index(String),
  #[error("rpc error: {0}")]
  Rpc(#[from] rpc::RpcError),
}

thread_local! {
  static OUTPOINT_TO_RUNE_BALANCES: RefCell<SHashMap<OutPointValue, SVec<RuneBalance>>> = RefCell::new(SHashMap::new());
  static RUNE_ID_TO_RUNE_ENTRY: RefCell<SHashMap<RuneId, RuneEntry>> = RefCell::new(SHashMap::new());
  static RUNE_TO_RUNE_ID: RefCell<SHashMap<u128, RuneId>> = RefCell::new(SHashMap::new());
  static TRANSACTION_ID_TO_RUNE: RefCell<SHashMap<TxidValue, u128>> = RefCell::new(SHashMap::new());
  static HEIGHT_TO_HEADER: RefCell<SBTreeMap<u32, Header>> = RefCell::new(SBTreeMap::new());
}

pub(crate) fn outpoint_to_rune_balances<F, R>(f: F) -> R
where
  F: FnOnce(&mut SHashMap<OutPointValue, SVec<RuneBalance>>) -> R,
{
  crate::OUTPOINT_TO_RUNE_BALANCES.with_borrow_mut(|b| f(b))
}

pub(crate) fn rune_id_to_rune_entry<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<RuneId, RuneEntry>) -> R,
{
  crate::RUNE_ID_TO_RUNE_ENTRY.with_borrow_mut(|r| f(r))
}

pub(crate) fn rune_to_rune_id<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<u128, RuneId>) -> R,
{
  crate::RUNE_TO_RUNE_ID.with_borrow_mut(|r| f(r))
}

pub(crate) fn transaction_id_to_rune<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<TxidValue, u128>) -> R,
{
  crate::TRANSACTION_ID_TO_RUNE.with_borrow_mut(|t| f(t))
}
