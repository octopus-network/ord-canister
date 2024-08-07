use crate::{index::entry::Entry, OrdError, OutPoint, RuneBalance, Txid};
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query};
use std::ops::Deref;
use std::str::FromStr;

#[query]
pub fn get_runes_by_utxo(txid: String, vout: u32) -> Result<Vec<RuneBalance>, OrdError> {
  let k = OutPoint::store(OutPoint {
    txid: Txid::from_str(&txid).map_err(|e| OrdError::Params(e.to_string()))?,
    vout,
  });
  let v =
    crate::outpoint_to_rune_balances(|b| b.get(&k).map(|v| v.deref().iter().map(|i| *i).collect()))
      .unwrap_or_default();
  Ok(v)
}

#[query]
pub fn get_height() -> Result<(u32, String), OrdError> {
  let (height, hash) = crate::highest_block();
  Ok((height, hash.to_string()))
}

#[query]
pub fn rpc_transform(args: TransformArgs) -> HttpResponse {
  let headers = args
    .response
    .headers
    .into_iter()
    .filter(|h| crate::rpc::should_keep(h.name.as_str()))
    .collect::<Vec<_>>();
  HttpResponse {
    status: args.response.status.clone(),
    body: args.response.body.clone(),
    headers,
  }
}

#[query]
pub fn get_log_records(count: usize) -> Vec<String> {
  ic_log::take_memory_records(count)
}

#[init]
pub fn init(url: String) {
  crate::init_storage();
  crate::init_ic_log();
  crate::set_url(url);
  crate::index::init_rune();
  crate::index::sync(1);
}

#[pre_upgrade]
fn pre_upgrade() {
  crate::persistence();
}

#[post_upgrade]
fn post_upgrade() {
  crate::restore();
  crate::init_ic_log();
  crate::index::sync(1);
}

ic_cdk::export_candid!();
