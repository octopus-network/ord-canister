mod bitcoin_api;
mod canister;
mod ic_log;
mod index;
mod notifier;
mod rpc;

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
use core2::io::Cursor;
use ic_stable_memory::{
  collections::{SBTreeMap, SHashMap, SVec},
  SBox,
};
pub use index::entry::{RuneBalance, RuneEntry, RuneUpdate};
use ord_canister_interface::OrdError;
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use std::cell::RefCell;

pub(crate) type Result<T> = std::result::Result<T, OrdError>;

thread_local! {
  static OUTPOINT_TO_RUNE_BALANCES: RefCell<Option<SHashMap<OutPointValue, SVec<RuneBalance>>>> = RefCell::new(None);
  static RUNE_ID_TO_RUNE_ENTRY: RefCell<Option<SHashMap<RuneId, RuneEntry>>> = RefCell::new(None);
  static RUNE_TO_RUNE_ID: RefCell<Option<SHashMap<u128, RuneId>>> = RefCell::new(None);
  static TRANSACTION_ID_TO_RUNE: RefCell<Option<SHashMap<TxidValue, u128>>> = RefCell::new(None);
  static HEIGHT_TO_BLOCK_HASH: RefCell<Option<SBTreeMap<u32, [u8; 32]>>> = RefCell::new(None);
  static RPC_URL: RefCell<Option<SBox<String>>> = RefCell::new(None);

  static HEIGHT_TO_OUTPOINTS: RefCell<Option<SHashMap<u32, SVec<OutPointValue>>>> = RefCell::new(None);
  static OUTPOINT_TO_HEIGHT: RefCell<Option<SHashMap<OutPointValue, u32>>> = RefCell::new(None);
  static HEIGHT_TO_RUNE_UPDATES: RefCell<Option<SHashMap<u32, SVec<RuneUpdate>>>> = RefCell::new(None);
  static HEIGHT_TO_RUNE_IDS: RefCell<Option<SHashMap<u32, SVec<RuneId>>>> = RefCell::new(None);

  static SUBSCRIBERS: RefCell<Option<SVec<SBox<String>>>> = RefCell::new(None);
}

pub const FIRST_HEIGHT: u32 = 839999;
pub const FIRST_BLOCK_HASH: &'static str =
  "0000000000000000000172014ba58d66455762add0512355ad651207918494ab";

pub(crate) fn highest_block() -> (u32, BlockHash) {
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow(|h| {
    let (height, hash) = h
      .as_ref()
      .expect("not initialized")
      .iter()
      .rev()
      .next()
      .expect("not initialized");
    let mut buffer = Cursor::new(*hash);
    let hash = BlockHash::consensus_decode(&mut buffer).unwrap();
    (*height, hash)
  })
}
pub(crate) fn block_hash(height: u32) -> Option<BlockHash> {
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow(|h| {
    let hash = h.as_ref().expect("not initialized").get(&height)?;
    let mut buffer = Cursor::new(*hash);
    BlockHash::consensus_decode(&mut buffer).ok()
  })
}

pub(crate) fn increase_height(height: u32, hash: BlockHash) {
  let mut buffer = Cursor::new([0; 32]);
  hash
    .consensus_encode(&mut buffer)
    .expect("in-memory writers don't error");

  crate::HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| {
    h.as_mut()
      .expect("not initialized")
      .insert(height, buffer.into_inner())
      .expect("MemoryOverflow");
  });
}

pub(crate) fn get_url() -> String {
  crate::RPC_URL.with_borrow_mut(|r| {
    r.as_mut()
      .expect("not initialized")
      .with(|s| s.clone())
      .unwrap()
  })
}

pub(crate) fn set_url(url: String) {
  crate::RPC_URL.with_borrow_mut(|r| {
    let new_url = SBox::new(url).expect("MemoryOverflow");
    r.replace(new_url)
  });
}

pub(crate) fn outpoint_to_rune_balances<F, R>(f: F) -> R
where
  F: FnOnce(&mut SHashMap<OutPointValue, SVec<RuneBalance>>) -> R,
{
  crate::OUTPOINT_TO_RUNE_BALANCES.with_borrow_mut(|b| f(b.as_mut().expect("not initialized")))
}

pub(crate) fn rune_id_to_rune_entry<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<RuneId, RuneEntry>) -> R,
{
  crate::RUNE_ID_TO_RUNE_ENTRY.with_borrow_mut(|r| f(r.as_mut().expect("not initialized")))
}

pub(crate) fn rune_to_rune_id<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<u128, RuneId>) -> R,
{
  crate::RUNE_TO_RUNE_ID.with_borrow_mut(|r| f(r.as_mut().expect("not initialized")))
}

pub(crate) fn transaction_id_to_rune<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<TxidValue, u128>) -> R,
{
  crate::TRANSACTION_ID_TO_RUNE.with_borrow_mut(|t| f(t.as_mut().expect("not initialized")))
}

pub(crate) fn height_to_block_hash<F, R>(f: F) -> R
where
  F: Fn(&mut SBTreeMap<u32, [u8; 32]>) -> R,
{
  crate::HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| f(h.as_mut().expect("not initialized")))
}

pub(crate) fn height_to_outpoints<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<u32, SVec<OutPointValue>>) -> R,
{
  crate::HEIGHT_TO_OUTPOINTS.with_borrow_mut(|h| f(h.as_mut().expect("not initialized")))
}

pub(crate) fn outpoint_to_height<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<OutPointValue, u32>) -> R,
{
  crate::OUTPOINT_TO_HEIGHT.with_borrow_mut(|h| f(h.as_mut().expect("not initialized")))
}

pub(crate) fn height_to_rune_updates<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<u32, SVec<RuneUpdate>>) -> R,
{
  crate::HEIGHT_TO_RUNE_UPDATES.with_borrow_mut(|h| f(h.as_mut().expect("not initialized")))
}

pub(crate) fn height_to_rune_ids<F, R>(f: F) -> R
where
  F: Fn(&mut SHashMap<u32, SVec<RuneId>>) -> R,
{
  crate::HEIGHT_TO_RUNE_IDS.with_borrow_mut(|h| f(h.as_mut().expect("not initialized")))
}

pub(crate) fn subscribers<F, R>(f: F) -> R
where
  F: Fn(&mut SVec<SBox<String>>) -> R,
{
  crate::SUBSCRIBERS.with_borrow_mut(|s| f(s.as_mut().expect("not initialized")))
}
