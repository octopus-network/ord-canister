use candid::{CandidType, Deserialize};

#[derive(Debug, CandidType, Deserialize)]
pub struct RuneBalance {
  pub confirmations: u32,
  pub rune_id: String,
  pub amount: u128,
  pub divisibility: u8,
  pub symbol: Option<String>,
}

#[derive(Debug, CandidType, Deserialize)]
pub struct GetEtchingResult {
  pub confirmations: u32,
  pub rune_id: String,
}

#[derive(Debug, CandidType, Deserialize)]
pub struct Terms {
  pub amount: Option<u128>,
  pub cap: Option<u128>,
  pub height: (Option<u64>, Option<u64>),
  pub offset: (Option<u64>, Option<u64>),
}

#[derive(Debug, CandidType, Deserialize)]
pub struct RuneEntry {
  pub confirmations: u32,
  pub rune_id: String,
  pub block: u64,
  pub burned: u128,
  pub divisibility: u8,
  pub etching: String,
  pub mints: u128,
  pub number: u64,
  pub premine: u128,
  pub spaced_rune: String,
  pub symbol: Option<String>,
  pub terms: Option<Terms>,
  pub timestamp: u64,
  pub turbo: bool,
}

#[derive(Debug, CandidType, Deserialize)]
pub enum Error {
  MaxOutpointsExceeded,
}
