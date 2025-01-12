use crate::ic_log::*;
use crate::{index::entry::Entry, OutPoint, Txid};
use ic_canister_log::log;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, pre_upgrade, query, update};
use ic_stable_memory::SBox;
use ord_canister_interface::*;
use std::ops::Deref;
use std::str::FromStr;

pub const REQUIRED_CONFIRMATIONS: u32 = 4;

#[query]
pub fn get_runes_by_utxo(txid: String, vout: u32) -> Result<Vec<RuneBalance>, OrdError> {
  let k = OutPoint::store(OutPoint {
    txid: Txid::from_str(&txid).map_err(|e| OrdError::Params(e.to_string()))?,
    vout,
  });
  let (cur_height, _) = crate::highest_block();
  let height =
    crate::outpoint_to_height(|o| o.get(&k).map(|h| *h)).ok_or(OrdError::OutPointNotFound)?;

  if cur_height < height || cur_height - height < REQUIRED_CONFIRMATIONS - 1 {
    return Err(OrdError::NotEnoughConfirmations);
  }

  let v = crate::outpoint_to_rune_balances(|b| {
    b.get(&k)
      .map(|v| v.deref().iter().map(|i| (*i).into()).collect())
  })
  .unwrap_or_default();
  Ok(v)
}

#[query]
pub fn query_runes(outpoints: Vec<String>) -> Result<Vec<Option<Vec<OrdRuneBalance>>>, OrdError> {
  if outpoints.len() > 64 {
    return Err(OrdError::Params("Too many outpoints".to_string()));
  }

  let (cur_height, _) = crate::highest_block();
  let mut piles = Vec::new();

  for str_outpoint in outpoints {
    let outpoint = match OutPoint::from_str(&str_outpoint) {
      Ok(o) => o,
      Err(e) => {
        log!(WARNING, "Failed to parse outpoint {}: {}", str_outpoint, e);
        piles.push(None);
        continue;
      }
    };
    let k = OutPoint::store(outpoint);

    crate::outpoint_to_rune_balances(|b| match b.get(&k) {
      Some(balances) => {
        let mut confirmations = 99;

        if let Some(height) = crate::outpoint_to_height(|o| o.get(&k).map(|h| *h)) {
          confirmations = cur_height - height + 1;
        }
        let mut outpoint_balances = Vec::new();
        for balance in balances.iter() {
          let rune_id = balance.id;
          let rune_entry = crate::rune_id_to_rune_entry(|ritre| ritre.get(&rune_id).map(|e| (*e)));
          if let Some(rune_entry) = rune_entry {
            outpoint_balances.push(OrdRuneBalance {
              id: rune_id.to_string(),
              confirmations,
              amount: balance.balance,
              divisibility: rune_entry.divisibility,
              symbol: rune_entry.symbol.map(|c| c.to_string()),
            });
          } else {
            log!(CRITICAL, "Rune not found for outpoint {}", str_outpoint);
          }
        }
        piles.push(Some(outpoint_balances));
      }
      None => piles.push(None),
    });
  }

  Ok(piles)
}

#[query]
pub fn get_etching(txid: String) -> Result<Option<OrdEtching>, OrdError> {
  let txid = Txid::from_str(&txid).map_err(|e| OrdError::Params(e.to_string()))?;
  let etching = crate::index::get_etching(txid)?;
  let (cur_height, _) = crate::highest_block();
  Ok(etching.map(|(id, entry)| OrdEtching {
    rune_id: id.to_string(),
    confirmations: cur_height - entry.block as u32 + 1,
  }))
}

#[query]
pub fn get_rune_entry_by_rune_id(rune_id: String) -> Result<OrdRuneEntry, OrdError> {
  let rune_id =
    ordinals::RuneId::from_str(&rune_id).map_err(|e| OrdError::Params(e.to_string()))?;
  let rune_entry =
    crate::index::get_rune_entry_by_rune_id(rune_id).ok_or(OrdError::RuneNotFound)?;
  let (cur_height, _) = crate::highest_block();
  Ok(OrdRuneEntry {
    confirmations: cur_height - rune_entry.block as u32 + 1,
    block: rune_entry.block,
    burned: rune_entry.burned,
    divisibility: rune_entry.divisibility,
    etching: rune_entry.etching.to_string(),
    mints: rune_entry.mints,
    number: 0,
    premine: rune_entry.premine,
    spaced_rune: rune_entry.spaced_rune.to_string(),
    symbol: rune_entry.symbol.map(|c| c.to_string()),
    terms: rune_entry.terms.map(|t| OrdTerms {
      amount: t.amount,
      cap: t.cap,
      height: t.height,
      offset: t.offset,
    }),
    timestamp: rune_entry.timestamp,
    turbo: rune_entry.turbo,
  })
}

#[query]
pub fn get_height() -> Result<(u32, String), OrdError> {
  let (height, hash) = crate::highest_block();
  Ok((height, hash.to_string()))
}

#[query(hidden = true)]
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

#[update]
pub fn set_url(url: String) -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }
  crate::set_url(url);
  Ok(())
}

#[query]
pub fn get_subscribers() -> Vec<String> {
  crate::subscribers(|s| s.iter().map(|s| s.to_string()).collect())
}

#[update]
pub fn remove_subscriber(idx: usize) -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }
  let _ = crate::subscribers(|s| {
    s.remove(idx);
  });
  Ok(())
}

#[query(hidden = true)]
fn http_request(
  req: ic_canisters_http_types::HttpRequest,
) -> ic_canisters_http_types::HttpResponse {
  if ic_cdk::api::data_certificate().is_none() {
    ic_cdk::trap("update call rejected");
  }
  if req.path() == "/logs" {
    crate::ic_log::do_reply(req)
  } else {
    ic_canisters_http_types::HttpResponseBuilder::not_found().build()
  }
}

#[init]
pub fn init(url: String) {
  crate::init_storage();
  crate::set_url(url);
  crate::index::init_rune();
  crate::index::update_index();
}

#[pre_upgrade]
fn pre_upgrade() {
  crate::persistence();
}

#[post_upgrade]
fn post_upgrade() {
  crate::restore();
  crate::index::update_index();
}

ic_cdk::export_candid!();
