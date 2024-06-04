use ic_cdk::api::management_canister::http_request::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

pub use bitcoincore_rpc_json::*;

#[derive(Debug, Error)]
pub enum RpcError<'a, 'b> {
  #[error("IO error occured while calling {0} onto {1} due to {2}.")]
  Io(&'static str, Cow<'a, str>, Cow<'b, str>),
  #[error("Decoding response of {0} from {1} failed due to {2}.")]
  Decode(&'static str, Cow<'a, str>, Cow<'b, str>),
  #[error("Received an error of endpoint {0} from {1}: {2}.")]
  Endpoint(&'static str, Cow<'a, str>, Cow<'b, str>),
}

#[derive(Serialize, Debug)]
struct Payload {
  pub jsonrpc: &'static str,
  pub id: i64,
  pub method: &'static str,
  pub params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct Reply<R> {
  #[allow(dead_code)]
  pub jsonrpc: String,
  #[allow(dead_code)]
  pub id: i64,
  pub error: Option<ErrorMsg>,
  pub result: Option<R>,
}

#[derive(Deserialize, Debug)]
struct ErrorMsg {
  #[allow(dead_code)]
  code: i64,
  message: String,
}

pub(crate) async fn make_rpc<R>(
  url: impl ToString,
  endpoint: impl ToString,
  params: impl Into<serde_json::Value>,
) -> Result<R, RpcError>
where
  R: DeserializeOwned,
{
  let payload = Payload {
    jsonrpc: "2.0",
    id: 1,
    method: endpoint,
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
  // TODO max cycle ~ 1_000_000_000
  let (response,) = http_request(args, 1_000_000_000)
    .await
    .map_err(|(_, e)| RpcError::Io(endpoint, url.to_string(), e))?;
  let reply: Reply<R> = serde_json::from_slice(response.body.as_slice())
    .map_err(|e| RpcError::Decode(endpoint, url, e.to_string()))?;
  if reply.error.is_some() {
    return Err(RpcError::Endpoint(
      endpoint,
      url,
      reply.error.map(|e| e.message).unwrap(),
    ));
  }
  match reply.result {
    Some(result) => Ok(decoder(result).ok_or(RpcError::Decode(
      endpoint,
      url,
      "Decoding failed".to_string(),
    ))?),
    _ => Err(RpcError::Decode(endpoint, url, "No result".to_string())),
  }
}

// TODO
const URL: &'static str = "http://localhost:8332";

pub(crate) async fn get_block_hash(height: u64) -> Result<BlockHash, RpcError> {
  make_rpc(URL, "getblockhash", serde_json::json!([height])).await
}

pub(crate) async fn get_block_header(hash: BlockHash) -> Result<BlockHeader, RpcError> {
  make_rpc(
    URL,
    "getblockheader",
    serde_json::json!([format!("{:x}", hash), true]),
  )
  .await
}

pub(crate) async fn get_block(hash: BlockHash) -> Result<Block, RpcError> {
  let hex: String = make_rpc(
    URL,
    "getblock",
    serde_json::json!([format!("{:x}", hash), 0]),
  )
  .await?;
  consensus::encode::deserialize_hex(&hex).map_err(|e| RpcError::Decode("getblock", URL, e))
}
