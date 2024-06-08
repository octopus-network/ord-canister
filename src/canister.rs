use crate::{OrdError, RuneBalance};
use candid::{CandidType, Deserialize, Principal};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};

#[query]
pub fn get_runes_by_utxo(txid: String, vout: u32) -> Result<Vec<RuneBalance>, OrdError> {
  Ok(vec![])
}

#[query]
pub fn get_height() -> Result<(u32, String), OrdError> {
  let (height, hash) = crate::highest_block_hash();
  Ok((height, hash.to_string()))
}

#[init]
pub fn init(url: String) {
  crate::set_url(url);
  ic_stable_memory::stable_memory_init();
  crate::set_beginning_block();
  crate::index::sync(1);
}

#[pre_upgrade]
fn pre_upgrade() {
  ic_stable_memory::stable_memory_pre_upgrade().expect("MemoryOverflow");
}

#[post_upgrade]
fn post_upgrade() {
  ic_stable_memory::stable_memory_post_upgrade();
}

ic_cdk::export_candid!();
