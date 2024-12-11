use crate::index::*;
use candid::CandidType;
use core2::io::Cursor;
use ic_stable_memory::{AsFixedSizeBytes, StableType};
use ord_canister_interface::OrdError;

pub(crate) trait Entry: Sized {
  type Value;

  fn load(value: Self::Value) -> Self;

  fn store(self) -> Self::Value;
}

#[derive(Copy, Eq, PartialEq, Clone, Debug, CandidType)]
pub struct RuneBalance {
  pub id: RuneId,
  pub balance: u128,
}

impl Into<ord_canister_interface::RuneBalance> for RuneBalance {
  fn into(self) -> ord_canister_interface::RuneBalance {
    ord_canister_interface::RuneBalance {
      id: ord_canister_interface::RuneId {
        block: self.id.block,
        tx: self.id.tx,
      },
      balance: self.balance,
    }
  }
}

impl AsFixedSizeBytes for RuneBalance {
  type Buf = [u8; Self::SIZE];

  const SIZE: usize = 28;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    let mut offset = 0;
    self
      .id
      .as_fixed_size_bytes(&mut buf[offset..offset + RuneId::SIZE]);
    offset += RuneId::SIZE;
    self.balance.as_fixed_size_bytes(&mut buf[offset..]);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut offset = 0;
    let id = RuneId::from_fixed_size_bytes(&buf[offset..offset + RuneId::SIZE]);
    offset += RuneId::SIZE;
    let balance = u128::from_fixed_size_bytes(&buf[offset..]);
    Self { id, balance }
  }
}

impl StableType for RuneBalance {}

#[derive(Debug, Copy, Clone)]
pub struct RuneUpdate {
  pub id: RuneId,
  pub burned: u128,
  pub mints: u128,
}

impl AsFixedSizeBytes for RuneUpdate {
  type Buf = [u8; Self::SIZE];

  const SIZE: usize = RuneId::SIZE + 16 + 16;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    let mut offset = 0;
    self
      .id
      .as_fixed_size_bytes(&mut buf[offset..offset + RuneId::SIZE]);
    offset += RuneId::SIZE;
    self
      .burned
      .as_fixed_size_bytes(&mut buf[offset..offset + 16]);
    offset += 16;
    self.mints.as_fixed_size_bytes(&mut buf[offset..]);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut offset = 0;
    let id = RuneId::from_fixed_size_bytes(&buf[offset..offset + RuneId::SIZE]);
    offset += RuneId::SIZE;
    let burned = u128::from_fixed_size_bytes(&buf[offset..offset + 16]);
    offset += 16;
    let mints = u128::from_fixed_size_bytes(&buf[offset..]);
    Self { id, burned, mints }
  }
}

impl StableType for RuneUpdate {}

pub(crate) type HeaderValue = [u8; 80];

impl Entry for Header {
  type Value = HeaderValue;

  fn load(value: Self::Value) -> Self {
    consensus::encode::deserialize(&value).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut buffer = Cursor::new([0; 80]);
    let len = self
      .consensus_encode(&mut buffer)
      .expect("in-memory writers don't error");
    let buffer = buffer.into_inner();
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

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct RuneEntry {
  pub block: u64,
  pub burned: u128,
  pub divisibility: u8,
  pub etching: Txid,
  pub mints: u128,
  // pub number: u64,
  pub premine: u128,
  pub spaced_rune: SpacedRune,
  pub symbol: Option<char>,
  pub terms: Option<Terms>,
  pub timestamp: u64,
  pub turbo: bool,
}

impl AsFixedSizeBytes for RuneEntry {
  type Buf = [u8; Self::SIZE];

  const SIZE: usize = 8 + 16 + 1 + 32 + 16 + 16 + SpacedRune::SIZE + 5 + Terms::SIZE + 1 + 8 + 1;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    let mut offset = 0;
    self.block.as_fixed_size_bytes(&mut buf[offset..offset + 8]);
    offset += 8;
    self
      .burned
      .as_fixed_size_bytes(&mut buf[offset..offset + 16]);
    offset += 16;
    self
      .divisibility
      .as_fixed_size_bytes(&mut buf[offset..offset + 1]);
    offset += 1;
    self
      .etching
      .store()
      .as_fixed_size_bytes(&mut buf[offset..offset + 32]);
    offset += 32;
    self
      .mints
      .as_fixed_size_bytes(&mut buf[offset..offset + 16]);
    offset += 16;
    // self.number.as_fixed_size_bytes(&mut buf[offset..]);
    // offset += 8;
    self
      .premine
      .as_fixed_size_bytes(&mut buf[offset..offset + 16]);
    offset += 16;
    self
      .spaced_rune
      .as_fixed_size_bytes(&mut buf[offset..offset + SpacedRune::SIZE]);
    offset += SpacedRune::SIZE;
    self
      .symbol
      .as_fixed_size_bytes(&mut buf[offset..offset + 5]);
    offset += 5;
    self
      .terms
      .as_fixed_size_bytes(&mut buf[offset..offset + Terms::SIZE + 1]);
    offset += Terms::SIZE + 1;
    self
      .timestamp
      .as_fixed_size_bytes(&mut buf[offset..offset + 8]);
    offset += 8;
    self.turbo.as_fixed_size_bytes(&mut buf[offset..]);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut offset = 0;
    let block = u64::from_fixed_size_bytes(&buf[offset..offset + 8]);
    offset += 8;
    let burned = u128::from_fixed_size_bytes(&buf[offset..offset + 16]);
    offset += 16;
    let divisibility = u8::from_fixed_size_bytes(&buf[offset..offset + 1]);
    offset += 1;
    let etching = TxidValue::from_fixed_size_bytes(&buf[offset..offset + 32]);
    offset += 32;
    let mints = u128::from_fixed_size_bytes(&buf[offset..offset + 16]);
    offset += 16;
    // let number = u64::from_fixed_size_bytes(&buf[offset..offset + 8]);
    // offset += 8;
    let premine = u128::from_fixed_size_bytes(&buf[offset..offset + 16]);
    offset += 16;
    let spaced_rune = SpacedRune::from_fixed_size_bytes(&buf[offset..offset + SpacedRune::SIZE]);
    offset += SpacedRune::SIZE;
    let symbol = Option::<char>::from_fixed_size_bytes(&buf[offset..offset + 5]);
    offset += 5;
    let terms = Option::<Terms>::from_fixed_size_bytes(&buf[offset..offset + Terms::SIZE + 1]);
    offset += Terms::SIZE + 1;
    let timestamp = u64::from_fixed_size_bytes(&buf[offset..offset + 8]);
    offset += 8;
    let turbo = bool::from_fixed_size_bytes(&buf[offset..]);
    Self {
      block,
      burned,
      divisibility,
      etching: Txid::load(etching),
      mints,
      // number,
      premine,
      spaced_rune,
      symbol,
      terms,
      timestamp,
      turbo,
    }
  }
}

impl StableType for RuneEntry {}

impl RuneEntry {
  pub fn mintable(&self, height: u64) -> Result<u128> {
    let Some(terms) = self.terms else {
      return Err(OrdError::Index(MintError::Unmintable));
    };

    if let Some(start) = self.start() {
      if height < start {
        return Err(OrdError::Index(MintError::Start(start)));
      }
    }

    if let Some(end) = self.end() {
      if height >= end {
        return Err(OrdError::Index(MintError::End(end)));
      }
    }

    let cap = terms.cap.unwrap_or_default();

    if self.mints >= cap {
      return Err(OrdError::Index(MintError::Cap(cap)));
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

pub(crate) type RuneEntryValue = (
  u64,          // block
  u128,         // burned
  u8,           // divisibility
  (u128, u128), // etching
  u128,         // mints
  // u64,                     // number
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
      // number: 0,
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
      // number,
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
      // number,
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
      // self.number,
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

pub(crate) type RuneIdValue = (u64, u32);

impl Entry for RuneId {
  type Value = RuneIdValue;

  fn load((block, tx): Self::Value) -> Self {
    Self { block, tx }
  }

  fn store(self) -> Self::Value {
    (self.block, self.tx)
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Copy, Debug)]
pub(crate) struct OutPointValue([u8; 36]);

impl Entry for OutPoint {
  type Value = OutPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut Cursor::new(value.0)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 36];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    OutPointValue(value)
  }
}

impl AsFixedSizeBytes for OutPointValue {
  type Buf = [u8; 36];

  const SIZE: usize = 36;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    buf.copy_from_slice(&self.0);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut value = [0; 36];
    value.copy_from_slice(buf);
    Self(value)
  }
}

impl StableType for OutPointValue {}

// pub(crate) type TxOutValue = (
//   u64,     // value
//   Vec<u8>, // script_pubkey
// );

// impl Entry for TxOut {
//   type Value = TxOutValue;

//   fn load(value: Self::Value) -> Self {
//     Self {
//       value: value.0,
//       script_pubkey: ScriptBuf::from_bytes(value.1),
//     }
//   }

//   fn store(self) -> Self::Value {
//     (self.value, self.script_pubkey.to_bytes())
//   }
// }

pub(crate) type SatPointValue = [u8; 44];

impl Entry for SatPoint {
  type Value = SatPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut Cursor::new(value)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 44];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    value
  }
}

pub(crate) type SatRange = (u64, u64);

impl Entry for SatRange {
  type Value = [u8; 11];

  fn load([b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10]: Self::Value) -> Self {
    let raw_base = u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, 0]);

    // 51 bit base
    let base = raw_base & ((1 << 51) - 1);

    let raw_delta = u64::from_le_bytes([b6, b7, b8, b9, b10, 0, 0, 0]);

    // 33 bit delta
    let delta = raw_delta >> 3;

    (base, base + delta)
  }

  fn store(self) -> Self::Value {
    let base = self.0;
    let delta = self.1 - self.0;
    let n = u128::from(base) | u128::from(delta) << 51;
    n.to_le_bytes()[0..11].try_into().unwrap()
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct TxidValue(pub [u8; 32]);

impl Entry for Txid {
  type Value = TxidValue;

  fn load(value: Self::Value) -> Self {
    Txid::from_byte_array(value.0)
  }

  fn store(self) -> Self::Value {
    TxidValue(Txid::to_byte_array(self))
  }
}

impl AsFixedSizeBytes for TxidValue {
  type Buf = [u8; 32];

  const SIZE: usize = 32;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    buf.copy_from_slice(&self.0);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut value = [0; 32];
    value.copy_from_slice(buf);
    Self(value)
  }
}

impl StableType for TxidValue {}
