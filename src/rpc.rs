use crate::*;
use candid::CandidType;
use ic_cdk::api::management_canister::http_request::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

pub use bitcoincore_rpc_json::*;

#[derive(Debug, Error, CandidType)]
pub enum RpcError {
  #[error("IO error occured while calling {0} onto {1} due to {2}.")]
  Io(&'static str, String, String),
  #[error("Decoding response of {0} from {1} failed due to {2}.")]
  Decode(&'static str, String, String),
  #[error("Received an error of endpoint {0} from {1}: {2}.")]
  Endpoint(&'static str, String, String),
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

pub(crate) async fn make_rpc<R>(
  url: impl ToString,
  endpoint: &'static str,
  params: impl Into<serde_json::Value>,
) -> Result<R>
where
  R: for<'a> Deserialize<'a> + std::fmt::Debug,
{
  let payload = Payload {
    jsonrpc: "1.0",
    id: "btc0",
    method: endpoint.as_ref(),
    params: params.into(),
  };
  let body = serde_json::to_vec(&payload).unwrap();
  let args = CanisterHttpRequestArgument {
    url: url.to_string(),
    method: HttpMethod::POST,
    body: Some(body),
    max_response_bytes: None,
    transform: None,
    headers: vec![
      HttpHeader {
        name: "Content-Type".to_string(),
        value: "application/json".to_string(),
      },
      HttpHeader {
        name: "User-Agent".to_string(),
        value: format!("omnity_ord_canister/{}", env!("CARGO_PKG_VERSION")),
      },
    ],
  };
  // TODO max cycle ~ 1000_000_000_000
  let (response,) = http_request(args, 1_000_000_000_000)
    .await
    .map_err(|(_, e)| OrdError::Rpc(RpcError::Io(endpoint.as_ref(), url.to_string(), e)))?;
  let reply: Reply<R> = serde_json::from_slice(response.body.as_slice()).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      endpoint.as_ref(),
      url.to_string(),
      e.to_string(),
    ))
  })?;
  ic_cdk::println!("{:?}", reply);
  if reply.error.is_some() {
    return Err(OrdError::Rpc(RpcError::Endpoint(
      endpoint.as_ref(),
      url.to_string(),
      reply.error.map(|e| e.message).unwrap(),
    )));
  }
  reply.result.ok_or(OrdError::Rpc(RpcError::Decode(
    endpoint.as_ref(),
    url.to_string(),
    "No result".to_string(),
  )))
}

pub(crate) async fn get_block_hash(url: &str, height: u32) -> Result<BlockHash> {
  let r = make_rpc::<String>(url, "getblockhash", serde_json::json!([height])).await?;
  let hash = BlockHash::from_str(&r).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getblockhash",
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
  )
  .await
}

pub(crate) async fn get_best_block_hash(url: &str) -> Result<BlockHash> {
  let r = make_rpc::<String>(url, "getbestblockhash", serde_json::json!([])).await?;
  let hash = BlockHash::from_str(&r).map_err(|e| {
    OrdError::Rpc(RpcError::Decode(
      "getbestblockhash",
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
  )
  .await?;
  use hex::FromHex;
  let hex = <Vec<u8>>::from_hex(hex)
    .map_err(|e| OrdError::Rpc(RpcError::Decode("getblock", url.to_string(), e.to_string())))?;
  consensus::encode::deserialize(&hex)
    .map_err(|e| OrdError::Rpc(RpcError::Decode("getblock", url.to_string(), e.to_string())))
}

pub(crate) async fn get_block_info(url: &str, hash: BlockHash) -> Result<GetBlockResult> {
  make_rpc::<GetBlockResult>(
    url,
    "getblock",
    serde_json::json!([format!("{:x}", hash), 1]),
  )
  .await
}
