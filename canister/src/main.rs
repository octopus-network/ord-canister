use bitcoin::block::Header;
use bitcoin::{OutPoint, Txid};
use candid::CandidType;
use candid::Deserialize;
use candid::{candid_method, Principal};
use ic_canister_log::log;
use ic_cdk::api::management_canister::http_request::{HttpResponse, TransformArgs};
use ic_cdk_macros::{init, post_upgrade, query, update};
use ordinals::Rune;
use ordinals::RuneId;
use runes_indexer::config::RunesIndexerArgs;
use runes_indexer::index::entry::Entry;
use runes_indexer::index::entry::RuneBalances;
use runes_indexer::index::entry::RuneEntry;
use runes_indexer::logs::{CRITICAL, INFO, WARNING};
use runes_indexer_interface::{OrdError, OrdEtching, OrdRuneBalance, OrdRuneEntry, OrdTerms};
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
    if let Some(rune_balances) = runes_indexer::index::mem_get_outpoint_to_rune_balances(k) {
      if let Some(height) = runes_indexer::index::mem_get_outpoint_to_height(k) {
        let confirmations = cur_height - height + 1;

        let mut outpoint_balances = Vec::new();
        for rune_balance in rune_balances.balances.iter() {
          let rune_entry =
            runes_indexer::index::mem_get_rune_id_to_rune_entry(rune_balance.rune_id.store());
          if let Some(rune_entry) = rune_entry {
            outpoint_balances.push(OrdRuneBalance {
              id: rune_balance.rune_id.to_string(),
              confirmations,
              amount: rune_balance.balance,
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
  let rune_entry = runes_indexer::index::mem_get_rune_id_to_rune_entry(rune_id.store())
    .ok_or(OrdError::RuneNotFound)?;
  let cur_height = runes_indexer::index::mem_latest_block_height().expect("No block height found");
  Ok(OrdRuneEntry {
    confirmations: cur_height - rune_entry.block as u32 + 1,
    block: rune_entry.block,
    burned: rune_entry.burned,
    divisibility: rune_entry.divisibility,
    etching: rune_entry.etching.to_string(),
    mints: rune_entry.mints,
    number: rune_entry.number,
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

  runes_indexer::index::cancel_shutdown();
  let config = runes_indexer::index::mem_get_config();
  let _ = runes_indexer::index::updater::update_index(config.network, config.subcribers);

  Ok(())
}

#[update]
pub fn stop() -> Result<(), String> {
  let caller = ic_cdk::api::caller();
  if !ic_cdk::api::is_controller(&caller) {
    return Err("Not authorized".to_string());
  }

  runes_indexer::index::shut_down();
  log!(INFO, "Waiting for index thread to finish...");

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
  runes_indexer::index::mem_set_config(config).unwrap();

  Ok(())
}

#[query]
pub fn get_subscribers() -> Vec<Principal> {
  runes_indexer::index::mem_get_config().subcribers
}

#[query(hidden = true)]
fn http_request(
  req: ic_canisters_http_types::HttpRequest,
) -> ic_canisters_http_types::HttpResponse {
  if ic_cdk::api::data_certificate().is_none() {
    ic_cdk::trap("update call rejected");
  }
  if req.path() == "/logs" {
    runes_indexer::logs::do_reply(req)
  } else {
    ic_canisters_http_types::HttpResponseBuilder::not_found().build()
  }
}

#[init]
#[candid_method(init)]
fn init(runes_indexer_args: RunesIndexerArgs) {
  match runes_indexer_args {
    RunesIndexerArgs::Init(config) => {
      runes_indexer::index::mem_set_config(config).unwrap();
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
      runes_indexer::index::mem_set_config(config).unwrap();
    }
    None | Some(RunesIndexerArgs::Upgrade(None)) => {}
    _ => ic_cdk::trap(
      "Cannot upgrade the canister with an Init argument. Please provide an Upgrade argument.",
    ),
  }
}

#[derive(CandidType, Deserialize)]
struct LoadArgs {
  data: Vec<u8>,
}

#[update]
#[candid_method]
fn load(args: LoadArgs) -> Result<(), String> {
  let bytes = lz4::block::decompress(&args.data, None).map_err(|e| e.to_string())?;
  if bytes[0] == 200u8 {
    let m: Vec<(RuneId, RuneEntry)> =
      bincode::deserialize(&bytes[1..]).map_err(|e| e.to_string())?;
    m.iter().for_each(|(rune_id, rune_entry)| {
      runes_indexer::index::mem_insert_rune_id_to_rune_entry(rune_id.store(), *rune_entry);
    });
  } else if bytes[0] == 201u8 {
    let m: Vec<(Rune, RuneId)> = bincode::deserialize(&bytes[1..]).map_err(|e| e.to_string())?;
    m.iter().for_each(|(rune, rune_id)| {
      runes_indexer::index::mem_insert_rune_to_rune_id(rune.store(), rune_id.store());
    });
  } else if bytes[0] == 202u8 {
    let m: Vec<(Txid, u128)> = bincode::deserialize(&bytes[1..]).map_err(|e| e.to_string())?;
    m.iter().for_each(|(txid, rune)| {
      runes_indexer::index::mem_insert_transaction_id_to_rune(txid.store(), *rune);
    });
  } else if bytes[0] == 203u8 {
    let m: Vec<(OutPoint, RuneBalances, u32)> =
      bincode::deserialize(&bytes[1..]).map_err(|e| e.to_string())?;
    m.iter().for_each(|(outpoint, rune_balances, height)| {
      let op = outpoint.store();
      runes_indexer::index::mem_insert_outpoint_to_rune_balances(op, rune_balances.clone());
      runes_indexer::index::mem_insert_outpoint_to_height(op, *height);
    });
  } else if bytes[0] == 204u8 {
    let (reserved_runes, runes, (height, header)): (u64, u64, (u32, Header)) =
      bincode::deserialize(&bytes[1..]).map_err(|e| e.to_string())?;
    runes_indexer::index::mem_insert_statistic_reserved_runes(height, reserved_runes);
    runes_indexer::index::mem_insert_statistic_runes(height, runes);
    runes_indexer::index::mem_insert_block_header(height, header.store());
  }
  Ok(())
}

ic_cdk::export_candid!();

fn main() {}
