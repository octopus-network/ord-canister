mod canister;
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
use candid::CandidType;
use ic_stable_memory::collections::{SBTreeMap, SHashMap, SVec};
pub use index::entry::{RuneBalance, RuneEntry};
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use std::cell::RefCell;
use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, OrdError>;

#[derive(Debug, Error, CandidType)]
pub enum OrdError {
  #[error("overflow")]
  Overflow,
  #[error("block verification")]
  BlockVerification(u32),
  #[error("index error: {0}")]
  Index(runes::MintError),
  #[error("rpc error: {0}")]
  Rpc(#[from] rpc::RpcError),
}

thread_local! {
  static OUTPOINT_TO_RUNE_BALANCES: RefCell<SHashMap<OutPointValue, SVec<RuneBalance>>> = RefCell::new(SHashMap::new());
  static RUNE_ID_TO_RUNE_ENTRY: RefCell<SHashMap<RuneId, RuneEntry>> = RefCell::new(SHashMap::new());
  static RUNE_TO_RUNE_ID: RefCell<SHashMap<u128, RuneId>> = RefCell::new(SHashMap::new());
  static TRANSACTION_ID_TO_RUNE: RefCell<SHashMap<TxidValue, u128>> = RefCell::new(SHashMap::new());
  static HEIGHT_TO_BLOCK_HASH: RefCell<SBTreeMap<u32, [u8; 32]>> = RefCell::new(SBTreeMap::new());
  static RPC_URL: RefCell<String> = RefCell::new(String::default());
}

pub const REQUIRED_CONFIRMATIONS: u32 = 4;
pub const FIRST_HEIGHT: u32 = 840000;
pub const FIRST_BLOCK_HASH: [u8; 32] =
  hex_literal::hex!("0000000000000000000320283a032748cef8227873ff4872689bf23f1cda83a5");

pub(crate) fn highest_block_hash() -> (u32, BlockHash) {
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow(|h| {
    let (height, hash) = h.iter().rev().next().expect("not initialized");
    let hash =
      BlockHash::from_raw_hash(bitcoin::hashes::sha256d::Hash::from_slice(&*hash).unwrap());
    (*height, hash)
  })
}

pub(crate) fn increase_height(height: u32, hash: BlockHash) {
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| {
    h.insert(height, *hash.as_ref()).expect("MemoryOverflow");
  });
}

pub(crate) fn set_beginning_block() {
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| {
    h.insert(FIRST_HEIGHT, FIRST_BLOCK_HASH)
      .expect("MemoryOverflow");
  });
}

pub(crate) fn get_url() -> String {
  crate::RPC_URL.with_borrow(|r| r.clone())
}

pub(crate) fn set_url(url: String) {
  crate::RPC_URL.with_borrow_mut(|r| *r = url);
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
