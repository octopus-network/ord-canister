use super::*;
use ic_stable_memory::{AsFixedSizeBytes, StableType};

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Eq)]
pub struct Terms {
  pub amount: Option<u128>,
  pub cap: Option<u128>,
  pub height: (Option<u64>, Option<u64>),
  pub offset: (Option<u64>, Option<u64>),
}

impl AsFixedSizeBytes for Terms {
  type Buf = [u8; Self::SIZE];

  const SIZE: usize = 6 + 16 + 16 + 8 + 8 + 8 + 8;

  fn as_fixed_size_bytes(&self, buf: &mut [u8]) {
    let mut offset = 0;
    self
      .amount
      .as_fixed_size_bytes(&mut buf[offset..offset + 17]);
    offset += 17;
    self.cap.as_fixed_size_bytes(&mut buf[offset..offset + 17]);
    offset += 17;
    self
      .height
      .0
      .as_fixed_size_bytes(&mut buf[offset..offset + 9]);
    offset += 9;
    self
      .height
      .1
      .as_fixed_size_bytes(&mut buf[offset..offset + 9]);
    offset += 9;
    self
      .offset
      .0
      .as_fixed_size_bytes(&mut buf[offset..offset + 9]);
    offset += 9;
    self.offset.1.as_fixed_size_bytes(&mut buf[offset..]);
  }

  fn from_fixed_size_bytes(buf: &[u8]) -> Self {
    let mut offset = 0;
    let amount = Option::<u128>::from_fixed_size_bytes(&buf[offset..offset + 17]);
    offset += 17;
    let cap = Option::<u128>::from_fixed_size_bytes(&buf[offset..offset + 17]);
    offset += 17;
    let h0 = Option::<u64>::from_fixed_size_bytes(&buf[offset..offset + 9]);
    offset += 9;
    let h1 = Option::<u64>::from_fixed_size_bytes(&buf[offset..offset + 9]);
    offset += 9;
    let o0 = Option::<u64>::from_fixed_size_bytes(&buf[offset..offset + 9]);
    offset += 9;
    let o1 = Option::<u64>::from_fixed_size_bytes(&buf[offset..offset + 9]);

    Self {
      amount,
      cap,
      height: (h0, h1),
      offset: (o0, o1),
    }
  }
}

impl StableType for Terms {}
