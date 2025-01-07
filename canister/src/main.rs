use candid::{candid_method, Principal};
use ic_canister_log::log;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, query, update};
use runes_indexer::config::RunesIndexerArgs;
use runes_indexer::ic_log::*;
use runes_indexer::{index::Entry, OutPoint, Txid};
use runes_indexer_interface::*;
use std::str::FromStr;

#[query]
pub fn query_runes(outpoints: Vec<String>) -> Result<Vec<Option<Vec<OrdRuneBalance>>>, OrdError> {
  if outpoints.len() > 64 {
    return Err(OrdError::Params("Too many outpoints".to_string()));
  }

  let cur_height = runes_indexer::index::mem_latest_block_height().expect("No block height found");
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
    if let Some(balances) = runes_indexer::index::mem_get_rune_balances(k) {
      if let Some(height) = runes_indexer::index::mem_get_height_by_outpoint(k) {
        let confirmations = cur_height - height + 1;

        let mut outpoint_balances = Vec::new();
        for (_, rune_id, balance) in balances.iter() {
          let rune_entry = runes_indexer::index::mem_get_rune_entry(rune_id.store());
          if let Some(rune_entry) = rune_entry {
            outpoint_balances.push(OrdRuneBalance {
              id: rune_id.to_string(),
              confirmations,
              amount: *balance,
              divisibility: rune_entry.divisibility,
              symbol: rune_entry.symbol.map(|c| c.to_string()),
            });
          } else {
            log!(CRITICAL, "Rune not found for outpoint {}", str_outpoint);
          }
        }
        piles.push(Some(outpoint_balances));
      } else {
        log!(WARNING, "Height not found for outpoint {}", str_outpoint);
        piles.push(None);
      }
    } else {
      log!(
        WARNING,
        "Rune balances not found for outpoint {}",
        str_outpoint
      );
      piles.push(None);
    }
  }

  Ok(piles)
}

#[query]
pub fn get_etching(txid: String) -> Result<Option<OrdEtching>, OrdError> {
  let txid = Txid::from_str(&txid).map_err(|e| OrdError::Params(e.to_string()))?;
  let cur_height = runes_indexer::index::mem_latest_block_height().expect("No block height found");
  Ok(
    runes_indexer::index::mem_get_etching(txid).map(|(id, entry)| OrdEtching {
      rune_id: id.to_string(),
      confirmations: cur_height - entry.block as u32 + 1,
    }),
  )
}

#[query]
pub fn get_rune_entry_by_rune_id(rune_id: String) -> Result<OrdRuneEntry, OrdError> {
  let rune_id =
    ordinals::RuneId::from_str(&rune_id).map_err(|e| OrdError::Params(e.to_string()))?;
  let rune_entry =
    runes_indexer::index::mem_get_rune_entry_by_rune_id(rune_id).ok_or(OrdError::RuneNotFound)?;
  let cur_height = runes_indexer::index::mem_latest_block_height().expect("No block height found");
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
  let (height, hash) = runes_indexer::index::mem_latest_block().expect("No block found");
  Ok((height, hash.to_string()))
}

#[query(hidden = true)]
pub fn rpc_transform(args: TransformArgs) -> HttpResponse {
  let headers = args
    .response
    .headers
    .into_iter()
    .filter(|h| runes_indexer::rpc::should_keep(h.name.as_str()))
    .collect::<Vec<_>>();
  HttpResponse {
    status: args.response.status.clone(),
    body: args.response.body.clone(),
    headers,
  }
}

#[update]
pub fn start() -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }

  runes_indexer::index::updater::update_index(runes_indexer::index::mem_get_config().network);

  Ok(())
}

#[update]
pub fn set_bitcoin_rpc_url(url: String) -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }
  let mut config = runes_indexer::index::mem_get_config();
  config.bitcoin_rpc_url = url;
  runes_indexer::index::mem_set_config(config);

  Ok(())
}

#[query]
pub fn get_subscribers() -> Vec<Principal> {
  runes_indexer::index::mem_get_config().subcribers
}

#[update]
pub fn add_subscriber(canister_id: Principal) -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }
  let mut config = runes_indexer::index::mem_get_config();
  config.subcribers.push(canister_id);
  runes_indexer::index::mem_set_config(config);

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
    runes_indexer::ic_log::do_reply(req)
  } else {
    ic_canisters_http_types::HttpResponseBuilder::not_found().build()
  }
}

#[init]
#[candid_method(init)]
fn init(runes_indexer_args: RunesIndexerArgs) {
  match runes_indexer_args {
    RunesIndexerArgs::Init(config) => {
      runes_indexer::index::mem_set_config(config);
    }
    RunesIndexerArgs::Upgrade(_) => ic_cdk::trap(
      "Cannot initialize the canister with an Upgrade argument. Please provide an Init argument.",
    ),
  }
}

#[post_upgrade]
fn post_upgrade(runes_indexer_args: Option<RunesIndexerArgs>) {
  match runes_indexer_args {
    Some(RunesIndexerArgs::Upgrade(Some(upgrade_args))) => {
      let mut config = runes_indexer::index::mem_get_config();
      if let Some(bitcoin_rpc_url) = upgrade_args.bitcoin_rpc_url {
        config.bitcoin_rpc_url = bitcoin_rpc_url;
      }
      if let Some(subscribers) = upgrade_args.subcribers {
        config.subcribers = subscribers;
      }
      runes_indexer::index::mem_set_config(config);
    }
    None | Some(RunesIndexerArgs::Upgrade(None)) => {}
    _ => ic_cdk::trap(
      "Cannot upgrade the canister with an Init argument. Please provide an Upgrade argument.",
    ),
  }
}

ic_cdk::export_candid!();

fn main() {}
