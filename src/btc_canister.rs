use bitcoin::{block::Header, BlockHash};
use candid::{self, CandidType, Deserialize, Principal};
use core2::io::Cursor;
use ic_cdk::api::call::RejectionCode;
use ord_canister_interface::OrdError;

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

pub async fn get_block_hash(height: u32) -> crate::Result<Option<BlockHash>> {
  let req = GetBlockHeadersRequest {
    start_height: height,
    end_height: Some(height),
    network: Network::Mainnet,
  };

  match ic_cdk::call::<_, (GetBlockHeadersResponse,)>(*BTC, "bitcoin_get_block_headers", (req,))
    .await
  {
    Ok(response) => {
      let header_bytes = response
        .0
        .block_headers
        .first()
        .ok_or_else(|| OrdError::Params(format!("failed to get header at height: {}", height)))?;

      let mut buffer = Cursor::new(header_bytes);
      let header = <Header as bitcoin::consensus::Decodable>::consensus_decode(&mut buffer)
        .map_err(|_| {
          OrdError::Params(format!("failed to decode block hash at height: {}", height))
        })?;

      Ok(Some(header.block_hash()))
    }
    Err(err)
      if err.0 == RejectionCode::CanisterReject && err.1.contains("StartHeightDoesNotExist") =>
    {
      Ok(None)
    }
    Err(err) => Err(OrdError::Params(format!(
      "failed to bitcoin_get_block_headers: {:?}",
      err
    ))),
  }
}
