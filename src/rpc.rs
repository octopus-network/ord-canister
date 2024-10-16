use crate::{
  ic_log::{ERROR, INFO},
  *,
};
use ic_canister_log::log;
use ic_cdk::api::management_canister::http_request::*;
use rune_indexer_interface::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

pub use bitcoincore_rpc_json::*;

lazy_static::lazy_static! {
  static ref ESSENTIAL_HEADERS: std::collections::HashSet<String> = {
    let mut set = std::collections::HashSet::new();
    set.insert("content-length".to_string());
    set.insert("content-range".to_string());
    set
  };
}

pub fn should_keep(header: &str) -> bool {
  let h = header.to_ascii_lowercase();
  ESSENTIAL_HEADERS.contains(&h)
}

#[derive(Serialize, Debug)]
struct Payload {
  pub jsonrpc: &'static str,
  pub id: &'static str,
  pub method: &'static str,
  pub params: serde_json::Value,
}

#[derive(Deserialize, Serialize, Debug)]
struct Reply<R> {
  #[allow(dead_code)]
  pub id: String,
  pub error: Option<ErrorMsg>,
  pub result: Option<R>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ErrorMsg {
  #[allow(dead_code)]
  code: i64,
  message: String,
}

/// [   0..1023] + [1024..2047] + [2048..3071] = 3072
/// [start, end] + [start, end] + [start, end] = total
fn split(end: u64, total: u64, limit: u64) -> (u64, u64) {
  let start = end + 1;
  let end = if start + limit >= total {
    total - 1
  } else {
    start + limit - 1
  };
  (start, end)
}

fn partial_request(
  url: impl ToString,
  endpoint: &'static str,
  params: impl Into<serde_json::Value>,
  range: (u64, u64),
) -> CanisterHttpRequestArgument {
  let payload = Payload {
    jsonrpc: "1.0",
    id: "btc0",
    method: endpoint.as_ref(),
    params: params.into(),
  };
  let body = serde_json::to_vec(&payload).unwrap();
  let mut hasher = Sha256::new();
  hasher.update(&body);
  let uniq: [u8; 32] = hasher.finalize().into();
  let uniq = hex::encode(uniq[0..16].to_vec());
  CanisterHttpRequestArgument {
    url: url.to_string(),
    method: HttpMethod::POST,
    body: Some(body),
    max_response_bytes: None,
    transform: Some(TransformContext {
      function: TransformFunc(candid::Func {
        principal: ic_cdk::api::id(),
        method: "rpc_transform".to_string(),
      }),
      context: vec![],
    }),
    headers: vec![
      HttpHeader {
        name: "Content-Type".to_string(),
        value: "application/json".to_string(),
      },
      HttpHeader {
        name: "User-Agent".to_string(),
        value: format!("omnity_ord_canister/{}", env!("CARGO_PKG_VERSION")),
      },
      HttpHeader {
        name: "Idempotency-Key".to_string(),
        value: uniq.clone(),
      },
      HttpHeader {
        name: "x-cloud-trace-context".to_string(),
        value: uniq.clone(),
      },
      HttpHeader {
        name: "Range".to_string(),
        value: format!("bytes={}-{}", range.0, range.1),
      },
    ],
  }
}

const MAX_RESPONSE_BYTES: u64 = 1_995_000;

// pub(crate) fn estimate_cycles(req_len: usize, estimate_len: usize) -> u128 {
//   171_360_000 + (req_len as u128) * 13_600 + (estimate_len as u128) * 27_200
// }

async fn make_single_request(
  args: CanisterHttpRequestArgument,
  estimate_cycle: u128,
) -> Result<HttpResponse> {
  let mut retry = 0;
  let mut cycles = estimate_cycle;
  loop {
    let response = http_request(args.clone(), cycles).await;
    match response {
      Ok((response,)) => return Ok(response),
      Err((code, e)) => {
        retry += 1;
        // 0.01T
        cycles += 10_000_000_000;
        if retry > 3 {
          log!(
            ERROR,
            "rpc error: {:?} => {}; won't retry(exceeds retry limit)",
            code,
            e
          );
          break Err(OrdError::Rpc(RpcError::Io(
            "make_single_request".to_string(),
            "retry limit exceeded".to_string(),
            e,
          )));
        }
        log!(
          ERROR,
          "rpc error: {:?} => {}; will retry with extra {} cycles",
          code,
          e,
          cycles
        );
      }
    }
  }
}

// max(estimate_len) = 1024 * 1024
pub(crate) async fn make_rpc<R>(
  url: impl ToString,
  endpoint: &'static str,
  params: impl Into<serde_json::Value> + Clone,
  estimate_cycle: u128,
) -> Result<R>
where
  R: for<'a> Deserialize<'a> + std::fmt::Debug,
{
  let mut range = (0, MAX_RESPONSE_BYTES);
  let mut buf = Vec::<u8>::with_capacity(MAX_RESPONSE_BYTES as usize);
  loop {
    let args = partial_request(url.to_string(), endpoint, params.clone(), range);
    let response = make_single_request(args, estimate_cycle).await?;
    if response.status == candid::Nat::from(200u32) {
      buf.extend_from_slice(response.body.as_slice());
      break;
    }
    if let Some(new_range) = response
      .headers
      .iter()
      .find(|h| h.name.eq_ignore_ascii_case("Content-Range"))
      .map(|r| -> Option<(u64, u64)> {
        let r = r.value.trim_start_matches("bytes ");
        let range_and_total = r.split('/').collect::<Vec<&str>>();
        let total = range_and_total[1].parse::<u64>().ok()?;
        let range = range_and_total[0].split('-').collect::<Vec<&str>>();
        let end = range[1].parse::<u64>().ok()?;
        Some(split(end, total, MAX_RESPONSE_BYTES))
      })
      .flatten()
    {
      log!(INFO, "bytes range: {:?} => {:?}", range, new_range);
      range = new_range;
      buf.extend_from_slice(response.body.as_slice());
      if range.0 >= range.1 {
        break;
      }
    } else {
      // some unexpected behaviour since we are not going to compatible with all servers
      buf.extend_from_slice(response.body.as_slice());
      break;
    }
  }
  log!(INFO, "reading all {} bytes from rpc", buf.len());
  let reply: Reply<R> = serde_json::from_slice(&buf).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      endpoint.to_string(),
      url.to_string(),
      e.to_string(),
    ))
  })?;
  if reply.error.is_some() {
    return Err(OrdError::Rpc(RpcError::Endpoint(
      endpoint.to_string(),
      url.to_string(),
      reply.error.map(|e| e.message).unwrap(),
    )));
  }
  reply.result.ok_or(OrdError::Rpc(RpcError::Decode(
    endpoint.to_string(),
    url.to_string(),
    "No result".to_string(),
  )))
}

pub(crate) async fn get_block_hash(url: &str, height: u32) -> Result<BlockHash> {
  let r = make_rpc::<String>(
    url,
    "getblockhash",
    serde_json::json!([height]),
    20_950_923_600,
  )
  .await?;
  let hash = BlockHash::from_str(&r).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getblockhash".to_string(),
      url.to_string(),
      e.to_string(),
    ))
  })?;
  Ok(hash)
}

pub(crate) async fn get_block_header(url: &str, hash: BlockHash) -> Result<GetBlockHeaderResult> {
  make_rpc::<GetBlockHeaderResult>(
    url,
    "getblockheader",
    serde_json::json!([format!("{:x}", hash), true]),
    20_950_923_600,
  )
  .await
}

pub(crate) async fn get_best_block_hash(url: &str) -> Result<BlockHash> {
  let r = make_rpc::<String>(
    url,
    "getbestblockhash",
    serde_json::json!([]),
    20_950_923_600,
  )
  .await?;
  let hash = BlockHash::from_str(&r).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getbestblockhash".to_string(),
      url.to_string(),
      e.to_string(),
    ))
  })?;
  Ok(hash)
}

pub(crate) async fn get_block(url: &str, hash: BlockHash) -> Result<Block> {
  let hex: String = make_rpc(
    url,
    "getblock",
    serde_json::json!([format!("{:x}", hash), 0]),
    20_996_000_000,
  )
  .await?;
  use hex::FromHex;
  let hex = <Vec<u8>>::from_hex(hex).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getblock".to_string(),
      url.to_string(),
      e.to_string(),
    ))
  })?;
  consensus::encode::deserialize(&hex).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getblock".to_string(),
      url.to_string(),
      e.to_string(),
    ))
  })
}
