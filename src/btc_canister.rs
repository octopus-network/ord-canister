use bitcoin::{block::Header, BlockHash};
use candid::{self, CandidType, Deserialize, Principal};
use core2::io::Cursor;
use rune_indexer_interface::OrdError;

lazy_static::lazy_static! {
  pub static ref BTC: Principal = Principal::from_text("ghsi2-tqaaa-aaaan-aaaca-cai").unwrap();
}

type BlockHeight = u32;

#[derive(CandidType, Deserialize)]
pub enum Network {
  #[serde(rename = "mainnet")]
  Mainnet,
  #[serde(rename = "regtest")]
  Regtest,
  #[serde(rename = "testnet")]
  Testnet,
}

#[derive(CandidType, Deserialize)]
pub struct GetBlockHeadersRequest {
  pub start_height: BlockHeight,
  pub end_height: Option<BlockHeight>,
  pub network: Network,
}

#[derive(CandidType, Deserialize)]
pub struct GetBlockHeadersResponse {
  pub tip_height: BlockHeight,
  pub block_headers: Vec<Vec<u8>>,
}

pub async fn get_block_hash(height: u32) -> crate::Result<BlockHash> {
  let req = GetBlockHeadersRequest {
    start_height: height,
    end_height: None,
    network: Network::Mainnet,
  };
  let res: (GetBlockHeadersResponse,) = ic_cdk::call(*BTC, "bitcoin_get_block_headers", (req,))
    .await
    .map_err(|_| OrdError::Params("failed to retrieve header from btc_canister".to_string()))?;
  let header = res
    .0
    .block_headers
    .first()
    .map(|b| {
      let mut buffer = Cursor::new(b);
      <Header as bitcoin::consensus::Decodable>::consensus_decode(&mut buffer)
    })
    .ok_or_else(|| OrdError::Params("block not ready".to_string()))?;
  Ok(
    header
      .map_err(|_| {
        OrdError::Params(
          "invalid block header from canister because we can't decode it".to_string(),
        )
      })?
      .block_hash(),
  )
}
