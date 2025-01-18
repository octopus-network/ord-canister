use super::*;
use ic_stable_structures::storable::{Bound, Storable};
use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum MintError {
  Cap(u128),
  End(u64),
  Start(u64),
  Unmintable,
}

impl Display for MintError {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      MintError::Cap(cap) => write!(f, "limited to {cap} mints"),
      MintError::End(end) => write!(f, "mint ended on block {end}"),
      MintError::Start(start) => write!(f, "mint starts on block {start}"),
      MintError::Unmintable => write!(f, "not mintable"),
    }
  }
}

pub trait Entry: Sized {
  type Value;

  fn load(value: Self::Value) -> Self;

  fn store(self) -> Self::Value;
}

pub(super) type HeaderValue = [u8; 80];

impl Entry for Header {
  type Value = HeaderValue;

  fn load(value: Self::Value) -> Self {
    consensus::encode::deserialize(&value).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut buffer = [0; 80];
    let len = self
      .consensus_encode(&mut buffer.as_mut_slice())
      .expect("in-memory writers don't error");
    debug_assert_eq!(len, buffer.len());
    buffer
  }
}

impl Entry for Rune {
  type Value = u128;

  fn load(value: Self::Value) -> Self {
    Self(value)
  }

  fn store(self) -> Self::Value {
    self.0
  }
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct RuneEntry {
  pub block: u64,
  pub burned: u128,
  pub divisibility: u8,
  pub etching: Txid,
  pub mints: u128,
  pub number: u64,
  pub premine: u128,
  pub spaced_rune: SpacedRune,
  pub symbol: Option<char>,
  pub terms: Option<Terms>,
  pub timestamp: u64,
  pub turbo: bool,
}

impl RuneEntry {
  pub fn mintable(&self, height: u64) -> Result<u128, MintError> {
    let Some(terms) = self.terms else {
      return Err(MintError::Unmintable);
    };

    if let Some(start) = self.start() {
      if height < start {
        return Err(MintError::Start(start));
      }
    }

    if let Some(end) = self.end() {
      if height >= end {
        return Err(MintError::End(end));
      }
    }

    let cap = terms.cap.unwrap_or_default();

    if self.mints >= cap {
      return Err(MintError::Cap(cap));
    }

    Ok(terms.amount.unwrap_or_default())
  }

  pub fn supply(&self) -> u128 {
    self.premine
      + self.mints
        * self
          .terms
          .and_then(|terms| terms.amount)
          .unwrap_or_default()
  }

  pub fn max_supply(&self) -> u128 {
    self.premine
      + self.terms.and_then(|terms| terms.cap).unwrap_or_default()
        * self
          .terms
          .and_then(|terms| terms.amount)
          .unwrap_or_default()
  }

  pub fn pile(&self, amount: u128) -> Pile {
    Pile {
      amount,
      divisibility: self.divisibility,
      symbol: self.symbol,
    }
  }

  pub fn start(&self) -> Option<u64> {
    let terms = self.terms?;

    let relative = terms
      .offset
      .0
      .map(|offset| self.block.saturating_add(offset));

    let absolute = terms.height.0;

    relative
      .zip(absolute)
      .map(|(relative, absolute)| relative.max(absolute))
      .or(relative)
      .or(absolute)
  }

  pub fn end(&self) -> Option<u64> {
    let terms = self.terms?;

    let relative = terms
      .offset
      .1
      .map(|offset| self.block.saturating_add(offset));

    let absolute = terms.height.1;

    relative
      .zip(absolute)
      .map(|(relative, absolute)| relative.min(absolute))
      .or(relative)
      .or(absolute)
  }
}

type TermsEntryValue = (
  Option<u128>,               // cap
  (Option<u64>, Option<u64>), // height
  Option<u128>,               // amount
  (Option<u64>, Option<u64>), // offset
);

pub(super) type RuneEntryValue = (
  u64,                     // block
  u128,                    // burned
  u8,                      // divisibility
  (u128, u128),            // etching
  u128,                    // mints
  u64,                     // number
  u128,                    // premine
  (u128, u32),             // spaced rune
  Option<char>,            // symbol
  Option<TermsEntryValue>, // terms
  u64,                     // timestamp
  bool,                    // turbo
);

impl Default for RuneEntry {
  fn default() -> Self {
    Self {
      block: 0,
      burned: 0,
      divisibility: 0,
      etching: Txid::all_zeros(),
      mints: 0,
      number: 0,
      premine: 0,
      spaced_rune: SpacedRune::default(),
      symbol: None,
      terms: None,
      timestamp: 0,
      turbo: false,
    }
  }
}

impl Entry for RuneEntry {
  type Value = RuneEntryValue;

  fn load(
    (
      block,
      burned,
      divisibility,
      etching,
      mints,
      number,
      premine,
      (rune, spacers),
      symbol,
      terms,
      timestamp,
      turbo,
    ): RuneEntryValue,
  ) -> Self {
    Self {
      block,
      burned,
      divisibility,
      etching: {
        let low = etching.0.to_le_bytes();
        let high = etching.1.to_le_bytes();
        Txid::from_byte_array([
          low[0], low[1], low[2], low[3], low[4], low[5], low[6], low[7], low[8], low[9], low[10],
          low[11], low[12], low[13], low[14], low[15], high[0], high[1], high[2], high[3], high[4],
          high[5], high[6], high[7], high[8], high[9], high[10], high[11], high[12], high[13],
          high[14], high[15],
        ])
      },
      mints,
      number,
      premine,
      spaced_rune: SpacedRune {
        rune: Rune(rune),
        spacers,
      },
      symbol,
      terms: terms.map(|(cap, height, amount, offset)| Terms {
        cap,
        height,
        amount,
        offset,
      }),
      timestamp,
      turbo,
    }
  }

  fn store(self) -> Self::Value {
    (
      self.block,
      self.burned,
      self.divisibility,
      {
        let bytes = self.etching.to_byte_array();
        (
          u128::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
          ]),
          u128::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
          ]),
        )
      },
      self.mints,
      self.number,
      self.premine,
      (self.spaced_rune.rune.0, self.spaced_rune.spacers),
      self.symbol,
      self.terms.map(
        |Terms {
           cap,
           height,
           amount,
           offset,
         }| (cap, height, amount, offset),
      ),
      self.timestamp,
      self.turbo,
    )
  }
}

impl Storable for RuneEntry {
  fn to_bytes(&self) -> Cow<[u8]> {
    let vec = bincode::serialize(self).unwrap();
    Cow::Owned(vec)
  }

  fn from_bytes(bytes: Cow<[u8]>) -> Self {
    bincode::deserialize(&bytes).unwrap()
  }

  const BOUND: Bound = Bound::Bounded {
    max_size: 307,
    is_fixed_size: false,
  };
}

pub(super) type RuneIdValue = (u64, u32);

impl Entry for RuneId {
  type Value = RuneIdValue;

  fn load((block, tx): Self::Value) -> Self {
    Self { block, tx }
  }

  fn store(self) -> Self::Value {
    (self.block, self.tx)
  }
}

pub(super) type OutPointValue = [u8; 36];

impl Entry for OutPoint {
  type Value = OutPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut bitcoin::io::Cursor::new(value)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 36];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    value
  }
}

pub(super) type SatPointValue = [u8; 44];

impl Entry for SatPoint {
  type Value = SatPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut bitcoin::io::Cursor::new(value)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 44];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    value
  }
}

pub(super) type TxidValue = [u8; 32];

impl Entry for Txid {
  type Value = TxidValue;

  fn load(value: Self::Value) -> Self {
    Txid::from_byte_array(value)
  }

  fn store(self) -> Self::Value {
    Txid::to_byte_array(self)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneBalance {
  pub rune_id: RuneId,
  pub balance: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuneBalances {
  pub balances: Vec<RuneBalance>,
}

impl Storable for RuneBalances {
  fn to_bytes(&self) -> Cow<[u8]> {
    let vec = bincode::serialize(self).unwrap();
    Cow::Owned(vec)
  }

  fn from_bytes(bytes: Cow<[u8]>) -> Self {
    bincode::deserialize(&bytes).unwrap()
  }

  const BOUND: Bound = Bound::Unbounded;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeRecord {
  pub removed_outpoints: Vec<(OutPoint, RuneBalances, u32)>,
  pub added_outpoints: Vec<OutPoint>,
  pub burned: HashMap<RuneId, u128>,
  pub mints: HashMap<RuneId, u128>,
  pub added_runes: Vec<(Rune, RuneId, Txid)>,
}

impl ChangeRecord {
  pub fn new() -> Self {
    Self {
      removed_outpoints: Vec::new(),
      added_outpoints: Vec::new(),
      burned: HashMap::new(),
      mints: HashMap::new(),
      added_runes: Vec::new(),
    }
  }
}

impl Storable for ChangeRecord {
  fn to_bytes(&self) -> Cow<[u8]> {
    let vec = bincode::serialize(self).unwrap();
    Cow::Owned(vec)
  }

  fn from_bytes(bytes: Cow<[u8]>) -> Self {
    bincode::deserialize(&bytes).unwrap()
  }

  const BOUND: Bound = Bound::Unbounded;
}
