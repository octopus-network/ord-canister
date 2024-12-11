use self::{entry::Entry, event::Event, lot::Lot};
use super::*;
use crate::ic_log::*;
use bitcoin::block::Header;
use ord_canister_interface::MintError;
use std::collections::BTreeMap;
use std::str::FromStr;

pub use self::entry::RuneEntry;
pub use updater::update_index;

pub(crate) mod entry;
pub mod event;
mod lot;
mod reorg;
mod updater;

#[allow(dead_code)]
pub const SCHEMA_VERSION: u64 = 26;

fn set_beginning_block() {
  let hash = BlockHash::from_str(FIRST_BLOCK_HASH).expect("valid hash");
  crate::increase_height(FIRST_HEIGHT, hash);
}

pub(crate) fn init_rune() {
  set_beginning_block();
  let rune = Rune(2055900680524219742);

  let id = RuneId { block: 1, tx: 0 };
  let etching = Txid::all_zeros();

  rune_to_rune_id(|r| r.insert(rune.store(), id)).expect("MemoryOverflow");

  rune_id_to_rune_entry(|r| {
    r.insert(
      id,
      RuneEntry {
        block: id.block,
        burned: 0,
        divisibility: 0,
        etching,
        terms: Some(Terms {
          amount: Some(1),
          cap: Some(u128::MAX),
          height: (
            Some((SUBSIDY_HALVING_INTERVAL * 4).into()),
            Some((SUBSIDY_HALVING_INTERVAL * 5).into()),
          ),
          offset: (None, None),
        }),
        mints: 0,
        premine: 0,
        spaced_rune: SpacedRune { rune, spacers: 128 },
        symbol: Some('\u{29C9}'),
        timestamp: 0,
        turbo: true,
      },
    )
  })
  .expect("MemoryOverflow");

  transaction_id_to_rune(|t| t.insert(Txid::store(etching), rune.store())).expect("MemoryOverflow");
}

#[allow(dead_code)]
pub(crate) fn get_etching(txid: Txid) -> Result<Option<SpacedRune>> {
  let Some(rune) = crate::transaction_id_to_rune(|t| t.get(&Txid::store(txid)).map(|r| *r)) else {
    return Ok(None);
  };

  let id = crate::rune_to_rune_id(|r| *r.get(&rune).unwrap());

  let entry = crate::rune_id_to_rune_entry(|r| *r.get(&id).unwrap());

  Ok(Some(entry.spaced_rune))
}

#[allow(dead_code)]
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
