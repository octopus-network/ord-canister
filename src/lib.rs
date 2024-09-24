#[cfg(feature = "cmp-header")]
mod btc_canister;
mod canister;
mod ic_log;
mod index;
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
pub use index::entry::{RuneBalance, RuneEntry};
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use rune_indexer_interface::OrdError;
use std::cell::RefCell;

pub(crate) type Result<T> = std::result::Result<T, OrdError>;

thread_local! {
  static OUTPOINT_TO_RUNE_BALANCES: RefCell<Option<SHashMap<OutPointValue, SVec<RuneBalance>>>> = RefCell::new(None);
  static RUNE_ID_TO_RUNE_ENTRY: RefCell<Option<SHashMap<RuneId, RuneEntry>>> = RefCell::new(None);
  static RUNE_TO_RUNE_ID: RefCell<Option<SHashMap<u128, RuneId>>> = RefCell::new(None);
  static TRANSACTION_ID_TO_RUNE: RefCell<Option<SHashMap<TxidValue, u128>>> = RefCell::new(None);
  static HEIGHT_TO_BLOCK_HASH: RefCell<Option<SBTreeMap<u32, [u8; 32]>>> = RefCell::new(None);
  static RPC_URL: RefCell<Option<SBox<String>>> = RefCell::new(None);
}

pub const REQUIRED_CONFIRMATIONS: u32 = 4;
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

pub(crate) fn init_storage() {
  ic_stable_memory::stable_memory_init();
  RPC_URL.with_borrow_mut(|r| r.replace(SBox::new("".to_string()).expect("MemoryOverflow")));
  OUTPOINT_TO_RUNE_BALANCES.with_borrow_mut(|b| b.replace(SHashMap::new()));
  RUNE_ID_TO_RUNE_ENTRY.with_borrow_mut(|r| r.replace(SHashMap::new()));
  RUNE_TO_RUNE_ID.with_borrow_mut(|r| r.replace(SHashMap::new()));
  TRANSACTION_ID_TO_RUNE.with_borrow_mut(|t| t.replace(SHashMap::new()));
  HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| h.replace(SBTreeMap::new()));
}

pub(crate) fn persistence() {
  let rpc_url: SBox<String> = RPC_URL.with(|l| l.take().unwrap());
  let boxed_rpc_url = SBox::new(rpc_url).expect("MemoryOverflow");
  let outpoint_to_balances: SHashMap<OutPointValue, SVec<RuneBalance>> =
    OUTPOINT_TO_RUNE_BALANCES.with(|b| b.take().unwrap());
  let boxed_outpoint_to_balances = SBox::new(outpoint_to_balances).expect("MemoryOverflow");
  let rune_id_to_rune_entry: SHashMap<RuneId, RuneEntry> =
    RUNE_ID_TO_RUNE_ENTRY.with(|r| r.borrow_mut().take().unwrap());
  let boxed_rune_id_to_rune_entry = SBox::new(rune_id_to_rune_entry).expect("MemoryOverflow");
  let run_to_rune_id: SHashMap<u128, RuneId> =
    RUNE_TO_RUNE_ID.with(|r| r.borrow_mut().take().unwrap());
  let boxed_rune_to_rune_id = SBox::new(run_to_rune_id).expect("MemoryOverflow");
  let transaction_id_to_rune: SHashMap<TxidValue, u128> =
    TRANSACTION_ID_TO_RUNE.with(|t| t.borrow_mut().take().unwrap());
  let boxed_transaction_id_to_rune = SBox::new(transaction_id_to_rune).expect("MemoryOverflow");
  let height_to_block_hash: SBTreeMap<u32, [u8; 32]> =
    HEIGHT_TO_BLOCK_HASH.with(|h| h.borrow_mut().take().unwrap());
  let boxed_height_to_block_hash = SBox::new(height_to_block_hash).expect("MemoryOverflow");
  ic_stable_memory::store_custom_data(0, boxed_rpc_url);
  ic_stable_memory::store_custom_data(1, boxed_outpoint_to_balances);
  ic_stable_memory::store_custom_data(2, boxed_rune_id_to_rune_entry);
  ic_stable_memory::store_custom_data(3, boxed_rune_to_rune_id);
  ic_stable_memory::store_custom_data(4, boxed_transaction_id_to_rune);
  ic_stable_memory::store_custom_data(5, boxed_height_to_block_hash);
  ic_stable_memory::stable_memory_pre_upgrade().expect("MemoryOverflow");
}

pub(crate) fn restore() {
  ic_stable_memory::stable_memory_post_upgrade();
  let rpc_url = ic_stable_memory::retrieve_custom_data::<SBox<String>>(0).unwrap();
  let outpoint_to_rune_balances =
    ic_stable_memory::retrieve_custom_data::<SHashMap<OutPointValue, SVec<RuneBalance>>>(1)
      .unwrap();
  let rune_id_to_rune_entry =
    ic_stable_memory::retrieve_custom_data::<SHashMap<RuneId, RuneEntry>>(2).unwrap();
  let run_to_rune_id = ic_stable_memory::retrieve_custom_data::<SHashMap<u128, RuneId>>(3).unwrap();
  let transaction_id_to_rune =
    ic_stable_memory::retrieve_custom_data::<SHashMap<TxidValue, u128>>(4).unwrap();
  let height_to_block_hash =
    ic_stable_memory::retrieve_custom_data::<SBTreeMap<u32, [u8; 32]>>(5).unwrap();
  RPC_URL.with_borrow_mut(|r| r.replace(rpc_url.into_inner()));
  OUTPOINT_TO_RUNE_BALANCES.with_borrow_mut(|b| b.replace(outpoint_to_rune_balances.into_inner()));
  RUNE_ID_TO_RUNE_ENTRY.with_borrow_mut(|r| r.replace(rune_id_to_rune_entry.into_inner()));
  RUNE_TO_RUNE_ID.with_borrow_mut(|r| r.replace(run_to_rune_id.into_inner()));
  TRANSACTION_ID_TO_RUNE.with_borrow_mut(|t| t.replace(transaction_id_to_rune.into_inner()));
  HEIGHT_TO_BLOCK_HASH.with_borrow_mut(|h| h.replace(height_to_block_hash.into_inner()));
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
  crate::RPC_URL.with_borrow_mut(|r| r.replace(SBox::new(url).expect("MemoryOverflow")));
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
