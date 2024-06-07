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

pub(crate) fn get_etching(txid: Txid) -> Result<Option<SpacedRune>> {
  let Some(rune) = crate::transaction_id_to_rune(|t| t.get(&Txid::store(txid)).map(|r| *r)) else {
    return Ok(None);
  };

  let id = crate::rune_to_rune_id(|r| *r.get(&rune).unwrap());

  let entry = crate::rune_id_to_rune_entry(|r| *r.get(&id).unwrap());

  Ok(Some(entry.spaced_rune))
}

pub(crate) fn get_rune_balances_for_output(
  outpoint: OutPoint,
) -> Result<BTreeMap<SpacedRune, Pile>> {
  crate::outpoint_to_rune_balances(|o| match o.get(&OutPoint::store(outpoint)) {
    Some(balances) => {
      let mut result = BTreeMap::new();
      for rune in balances.iter() {
        let rune = *rune;

        let entry = rune_id_to_rune_entry(|r| r.get(&rune.id).map(|r| *r).unwrap());

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
    None => Ok(BTreeMap::new()),
  })
}
