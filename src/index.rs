use {
  self::{
    entry::{
      Entry, HeaderValue, OutPointValue, RuneEntryValue, RuneIdValue, SatPointValue, SatRange,
      TxOutValue, TxidValue,
    },
    event::Event,
    lot::Lot,
  },
  super::{runes::MintError, *},
  bitcoin::block::Header,
  std::collections::BTreeMap,
};

pub use self::entry::RuneEntry;

pub(crate) mod entry;
pub mod event;
mod lot;
mod updater;

const SCHEMA_VERSION: u64 = 26;

pub(crate) fn encode_rune_balance(id: RuneId, balance: u128, buffer: &mut Vec<u8>) {
  varint::encode_to_vec(id.block.into(), buffer);
  varint::encode_to_vec(id.tx.into(), buffer);
  varint::encode_to_vec(balance, buffer);
}

pub(crate) fn get_etching(txid: Txid) -> Result<Option<SpacedRune>> {
  let Some(rune) = crate::transaction_id_to_rune(|t| t.get(&Txid::store(txid)).map(|r| *r)) else {
    return Ok(None);
  };

  let id = crate::rune_to_rune_id(|r| *r.get(&rune).unwrap());

  let entry = crate::rune_id_to_rune_entry(|r| *r.get(&id).unwrap());

  Ok(Some(entry.spaced_rune))
}

pub(crate) fn decode_rune_balance(buffer: &[u8]) -> Result<((RuneId, u128), usize)> {
  let mut len = 0;
  let (block, block_len) = varint::decode(&buffer[len..])?;
  len += block_len;
  let (tx, tx_len) = varint::decode(&buffer[len..])?;
  len += tx_len;
  let id = RuneId {
    block: block.try_into()?,
    tx: tx.try_into()?,
  };
  let (balance, balance_len) = varint::decode(&buffer[len..])?;
  len += balance_len;
  Ok(((id, balance), len))
}

pub(crate) fn get_rune_balances_for_output(
  outpoint: OutPoint,
) -> Result<BTreeMap<SpacedRune, Pile>> {
  // let outpoint_to_balances = rtx.open_table(OUTPOINT_TO_RUNE_BALANCES)?;

  // let id_to_rune_entries = rtx.open_table(RUNE_ID_TO_RUNE_ENTRY)?;

  let Some(balances) =
    crate::outpoint_to_rune_balances(|o| o.get(&OutPoint::store(outpoint)).map(|b| *b))
  else {
    return Ok(BTreeMap::new());
  };

  // let balances_buffer = balances.value();

  let mut result = BTreeMap::new();
  for rune in balances.iter() {
    let rune = *rune;
    // let ((id, amount), length) = decode_rune_balance(&balances_buffer[i..]).unwrap();

    let entry = rune_id_to_rune_entry(|r| r.get(id.store()).map(|r| *r).unwrap());

    result.insert(
      entry.spaced_rune,
      Pile {
        amount: rune.balance,
        divisibility: entry.divisibility,
        symbol: entry.symbol,
      },
    );
  }

  Ok(result)
}
