mod index;
mod rpc;

pub use bitcoin::{
  address::{Address, NetworkUnchecked},
  block::Header,
  blockdata::{
    constants::{DIFFCHANGE_INTERVAL, MAX_SCRIPT_ELEMENT_SIZE, SUBSIDY_HALVING_INTERVAL},
    locktime::absolute::LockTime,
  },
  consensus::{self, Decodable, Encodable},
  hash_types::{BlockHash, TxMerkleNode},
  hashes::Hash,
  script, Amount, Block, Network, OutPoint, Script, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
  Txid, Witness,
};
use ic_stable_memory::collections::{SHashMap, SVec};
pub use ordinals::{
  varint, Artifact, Charm, Edict, Epoch, Etching, Height, Pile, Rarity, Rune, RuneId, Runestone,
  Sat, SatPoint, SpacedRune, Terms,
};
use std::cell::RefCell;

pub(crate) type Result<T = (), E = anyhow::Error> = std::result::Result<T, E>;

thread_local! {
    // static SATPOINT_TO_SEQUENCE_NUMBER: RefCell<SHashMap<SatPointValue, SVec<u32>>> = RefCell::new(SHashMap::new());
    // static SAT_TO_SEQUENCE_NUMBER: RefCell<SHashMap<u64, SVec<u32>>> = RefCell::new(SHashMap::new());
    // static SEQUENCE_NUMBER_TO_CHILDREN: RefCell<SHashMap<u32, SVec<u32>>> = RefCell::new(SHashMap::new());
    // TODO
    // static SCRIPT_PUBKEY_TO_OUTPOINT: RefCell<SHashMap<&[u8], SVec<OutPointValue>>> = RefCell::new(SHashMap::new());
    // TODO
    // static CONTENT_TYPE_TO_COUNT: RefCell<SHashMap<Option<&[u8]>, u64>> = RefCell::new(SHashMap::new());
    // TODO
    // static HEIGHT_TO_BLOCK_HEADER: RefCell<SHashMap<u32, HeaderValue>> = RefCell::new(SHashMap::new());
    // static HEIGHT_TO_LAST_SEQUENCE_NUMBER: RefCell<SHashMap<u32, u32>> = RefCell::new(SHashMap::new());
    // static HOME_INSCRIPTIONS: RefCell<SHashMap<u32, InscriptionIdValue>> = RefCell::new(SHashMap::new());
    // static INSCRIPTION_ID_TO_SEQUENCE_NUMBER: RefCell<SHashMap<InscriptionIdValue, u32>> = RefCell::new(SHashMap::new());
    // static INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER: RefCell<SHashMap<i32, u32>> = RefCell::new(SHashMap::new());
    // balance = rune_id(block: u64, tx: u32) + balance: u128: bytes28
    // TODO define RuneBalance
    static OUTPOINT_TO_RUNE_BALANCES: RefCell<SHashMap<OutPoint, SVec<[u8; 28]>>> = RefCell::new(SHashMap::new());
    static RUNE_ID_TO_RUNE_ENTRY: RefCell<SHashMap<RuneId, RuneEntry>> = RefCell::new(SHashMap::new());
    static RUNE_TO_RUNE_ID: RefCell<SHashMap<u128, RuneId>> = RefCell::new(SHashMap::new());
    static TRANSACTION_ID_TO_RUNE: RefCell<SHashMap<Txid, u128>> = RefCell::new(SHashMap::new());

    // static OUTPOINT_TO_SAT_RANGES: RefCell<SHashMap<&OutPointValue, SVec<&[u8]>>> = RefCell::new(SHashMap::new());
    // static OUTPOINT_TO_TXOUT: RefCell<SHashMap<&OutPointValue, TxOutValue>> = RefCell::new(SHashMap::new());
    // static SAT_TO_SATPOINT: RefCell<SHashMap<u64, SVec<SatPointValue>>> = RefCell::new(SHashMap::new());
    // static SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY: RefCell<SHashMap<u32, InscriptionEntryValue>> = RefCell::new(SHashMap::new());
    // static SEQUENCE_NUMBER_TO_RUNE_ID: RefCell<SHashMap<u32, RuneIdValue>> = RefCell::new(SHashMap::new());
    // static SEQUENCE_NUMBER_TO_SATPOINT: RefCell<SHashMap<u32, SVec<SatPointValue>>> = RefCell::new(SHashMap::new());
    // static STATISTIC_TO_COUNT: RefCell<SHashMap<u64, u64>> = RefCell::new(SHashMap::new());
    // static TRANSACTION_ID_TO_TRANSACTION: RefCell<SHashMap<&TxidValue, &[u8]>> = RefCell::new(SHashMap::new());
    // static WRITE_TRANSACTION_STARTING_BLOCK_COUNT_TO_TIMESTAMP: RefCell<SHashMap<u32, u128>> = RefCell::new(SHashMap::new());
}

pub(crate) fn outpoint_to_rune_balances(f: F) -> R
where
  F: Fn(&mut SHashMap<OutPoint, RawRuneBalance>) -> R,
{
  crate::OUTPOINT_TO_RUNE_BALANCES.with_mut(|b| f(b))
}

pub(crate) fn rune_id_to_entry(f: F) -> R
where
  F: Fn(&mut SHashMap<RuneId, RuneEntry>) -> R,
{
  crate::RUNE_ID_TO_RUNE_ENTRY.with_mut(|r| f(r))
}

pub(crate) fn rune_to_rune_id(f: F) -> R
where
  F: Fn(&mut SHashMap<u128, RuneId>) -> R,
{
  crate::RUNE_TO_RUNE_ID.with_mut(|r| f(r))
}

pub(crate) fn transaction_id_to_rune(f: F) -> R
where
  F: Fn(&mut SHashMap<Txid, u128>) -> R,
{
  crate::TRANSACTION_ID_TO_RUNE.with_mut(|t| f(t))
}
