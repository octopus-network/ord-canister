pub use self::entry::{Entry, RuneEntry};
use self::lot::Lot;
use super::*;
use crate::config::Config;
use crate::index::entry::{
  HeaderValue, OutPointValue, OutPoints, RuneBalances, RuneIdValue, RuneUpdates, Runes, TxidValue,
};
use crate::logs::*;
use bitcoin::block::Header;
use ic_canister_log::log;
use ic_cdk::api::management_canister::bitcoin::BitcoinNetwork;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableCell};
use ordinals::RuneId;
use runes_indexer_interface::MintError;
use std::cell::RefCell;
use std::collections::HashMap;

pub mod entry;
mod lot;
// mod reorg;
pub mod updater;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
  static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
      RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

  static CONFIG: RefCell<StableCell<Config, Memory>> = RefCell::new(
      StableCell::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))),
          Config::default()
      ).unwrap()
  );

  // TODO: trim outdated headers
  static HEIGHT_TO_BLOCK_HEADER: RefCell<StableBTreeMap<u32, HeaderValue, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
      )
  );

  static OUTPOINT_TO_RUNE_BALANCES: RefCell<StableBTreeMap<OutPointValue, RuneBalances, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
      )
  );

  static RUNE_ID_TO_RUNE_ENTRY: RefCell<StableBTreeMap<RuneIdValue, RuneEntry, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))),
      )
  );

  static RUNE_TO_RUNE_ID: RefCell<StableBTreeMap<u128, RuneIdValue, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))),
      )
  );

  static TRANSACTION_ID_TO_RUNE: RefCell<StableBTreeMap<TxidValue, u128, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))),
      )
  );

  static OUTPOINT_TO_HEIGHT: RefCell<StableBTreeMap<OutPointValue, u32, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6))),
      )
  );

  static HEIGHT_TO_STATISTIC_RESERVED_RUNES: RefCell<StableBTreeMap<u32, u64, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(7))),
      )
  );

  static HEIGHT_TO_STATISTIC_RUNES: RefCell<StableBTreeMap<u32, u64, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(8))),
      )
  );

  static HEIGHT_TO_RUNES: RefCell<StableBTreeMap<u32, Runes, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(9))),
      )
  );

  static HEIGHT_TO_RUNE_UPDATES: RefCell<StableBTreeMap<u32, RuneUpdates, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(10))),
      )
  );

  static HEIGHT_TO_OUTPOINTS: RefCell<StableBTreeMap<u32, OutPoints, Memory>> = RefCell::new(
      StableBTreeMap::init(
          MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(11))),
      )
  );
}

pub fn mem_latest_block() -> Option<(u32, BlockHash)> {
  HEIGHT_TO_BLOCK_HEADER.with(|m| {
    m.borrow()
      .iter()
      .rev()
      .next()
      .map(|(height, header_value)| {
        let header = Header::load(header_value);
        (height, header.block_hash())
      })
  })
}

pub fn mem_latest_block_height() -> Option<u32> {
  HEIGHT_TO_BLOCK_HEADER.with(|m| m.borrow().iter().rev().next().map(|(height, _)| height))
}

pub fn mem_statistic_reserved_runes() -> u64 {
  HEIGHT_TO_STATISTIC_RESERVED_RUNES.with(|m| {
    m.borrow()
      .iter()
      .rev()
      .next()
      .map(|(_, runes)| runes)
      .unwrap_or(0)
  })
}

pub fn mem_insert_statistic_reserved_runes(height: u32, runes: u64) {
  HEIGHT_TO_STATISTIC_RESERVED_RUNES.with(|m| m.borrow_mut().insert(height, runes));
}

pub fn mem_statistic_runes() -> u64 {
  HEIGHT_TO_STATISTIC_RUNES.with(|m| {
    m.borrow()
      .iter()
      .rev()
      .next()
      .map(|(_, runes)| runes)
      .unwrap_or(0)
  })
}

pub fn mem_insert_statistic_runes(height: u32, runes: u64) {
  HEIGHT_TO_STATISTIC_RUNES.with(|m| m.borrow_mut().insert(height, runes));
}

pub fn mem_insert_block_header(height: u32, header_value: HeaderValue) {
  HEIGHT_TO_BLOCK_HEADER.with(|m| m.borrow_mut().insert(height, header_value));
}

pub fn mem_insert_rune_balances(outpoint_value: OutPointValue, rune_balances: RuneBalances) {
  OUTPOINT_TO_RUNE_BALANCES.with(|m| m.borrow_mut().insert(outpoint_value, rune_balances));
}

pub fn mem_get_rune_balances(outpoint_value: OutPointValue) -> Option<RuneBalances> {
  OUTPOINT_TO_RUNE_BALANCES.with(|m| m.borrow().get(&outpoint_value))
}

pub(crate) fn mem_remove_rune_balances(outpoint_value: OutPointValue) -> Option<RuneBalances> {
  OUTPOINT_TO_RUNE_BALANCES.with(|m| m.borrow_mut().remove(&outpoint_value))
}

pub fn mem_get_rune_entry(rune_id_value: RuneIdValue) -> Option<RuneEntry> {
  RUNE_ID_TO_RUNE_ENTRY.with(|m| m.borrow().get(&rune_id_value))
}

pub fn mem_insert_rune_entry(rune_id_value: RuneIdValue, rune_entry: RuneEntry) {
  RUNE_ID_TO_RUNE_ENTRY.with(|m| m.borrow_mut().insert(rune_id_value, rune_entry));
}

pub fn mem_length_rune_to_rune_id() -> u64 {
  RUNE_TO_RUNE_ID.with(|m| m.borrow().len())
}

pub fn mem_length_rune_entry() -> u64 {
  RUNE_ID_TO_RUNE_ENTRY.with(|m| m.borrow().len())
}

pub fn mem_length_rune_balance() -> u64 {
  OUTPOINT_TO_RUNE_BALANCES.with(|m| m.borrow().len())
}

pub fn mem_length_transaction_id_to_rune() -> u64 {
  TRANSACTION_ID_TO_RUNE.with(|m| m.borrow().len())
}

pub fn mem_length_outpoint_to_height() -> u64 {
  OUTPOINT_TO_HEIGHT.with(|m| m.borrow().len())
}

pub(crate) fn mem_get_rune_id(rune: u128) -> Option<RuneIdValue> {
  RUNE_TO_RUNE_ID.with(|m| m.borrow().get(&rune))
}

pub fn mem_insert_rune_id(rune: u128, rune_id_value: RuneIdValue) {
  RUNE_TO_RUNE_ID.with(|m| m.borrow_mut().insert(rune, rune_id_value));
}

pub fn mem_insert_transaction_id_to_rune(txid: TxidValue, rune: u128) {
  TRANSACTION_ID_TO_RUNE.with(|m| m.borrow_mut().insert(txid, rune));
}

pub fn mem_get_config() -> Config {
  CONFIG.with(|m| m.borrow().get().clone())
}

pub fn mem_set_config(config: Config) {
  CONFIG.with(|m| m.borrow_mut().set(config));
}

pub fn mem_get_height_by_outpoint(outpoint: OutPointValue) -> Option<u32> {
  OUTPOINT_TO_HEIGHT.with(|m| m.borrow().get(&outpoint))
}

pub fn mem_insert_height_for_outpoint(outpoint: OutPointValue, height: u32) {
  OUTPOINT_TO_HEIGHT.with(|m| m.borrow_mut().insert(outpoint, height));
}

pub fn mem_get_etching(txid: Txid) -> Option<(RuneId, RuneEntry)> {
  TRANSACTION_ID_TO_RUNE.with(|m| {
    m.borrow()
      .get(&Txid::store(txid))
      .and_then(|rune| RUNE_TO_RUNE_ID.with(|m| m.borrow().get(&rune)))
      .and_then(|id| {
        RUNE_ID_TO_RUNE_ENTRY.with(|m| m.borrow().get(&id).map(|e| (RuneId::load(id), e)))
      })
  })
}

pub fn mem_get_rune_entry_by_rune_id(rune_id: RuneId) -> Option<RuneEntry> {
  RUNE_ID_TO_RUNE_ENTRY.with(|m| m.borrow().get(&rune_id.store()))
}

pub fn init_mainnet() {
  let rune = Rune(2055900680524219742);

  let id = RuneId { block: 1, tx: 0 };
  let etching = Txid::all_zeros();

  mem_insert_rune_id(rune.store(), id.store());
  mem_insert_statistic_runes(1, 1);

  mem_insert_rune_entry(
    id.store(),
    RuneEntry {
      block: id.block,
      burned: 0,
      divisibility: 0,
      etching,
      terms: Some(Terms {
        amount: Some(1),
        cap: Some(u128::MAX),
        height: (
          Some((SUBSIDY_HALVING_INTERVAL * 4).into()),
          Some((SUBSIDY_HALVING_INTERVAL * 5).into()),
        ),
        offset: (None, None),
      }),
      mints: 0,
      number: 0,
      premine: 0,
      spaced_rune: SpacedRune { rune, spacers: 128 },
      symbol: Some('\u{29C9}'),
      timestamp: 0,
      turbo: true,
    },
  );

  mem_insert_transaction_id_to_rune(etching.store(), rune.store());
}

pub fn next_block(network: BitcoinNetwork) -> (u32, Option<BlockHash>) {
  mem_latest_block()
    .map(|(height, prev_blockhash)| (height + 1, Some(prev_blockhash)))
    .unwrap_or(match network {
      BitcoinNetwork::Mainnet => (Rune::first_rune_height(bitcoin::Network::Bitcoin), None),
      BitcoinNetwork::Testnet => (Rune::first_rune_height(bitcoin::Network::Testnet4), None),
      BitcoinNetwork::Regtest => (Rune::first_rune_height(bitcoin::Network::Regtest), None),
    })
}
