use {
  self::{entry::Entry, event::Event, lot::Lot},
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

pub(crate) async fn get_highest_from_rpc() -> Result<(u32, BlockHash)> {
  let url = get_url();
  let hash = rpc::get_best_block_hash(&url).await?;
  let header = rpc::get_block_header(&url, hash).await?;
  Ok((header.height.try_into().expect("usize to u32"), hash))
}

pub fn sync(secs: u64) {
  ic_cdk_timers::set_timer(std::time::Duration::from_secs(secs), || {
    ic_cdk::spawn(async move {
      let (height, current) = crate::highest_block_hash();
      match get_highest_from_rpc().await {
        Ok((best, hash)) => {
          ic_cdk::println!("our best = {}, their best = {}", height, best);
          if height + REQUIRED_CONFIRMATIONS > best {
            sync(60);
          } else {
            match updater::get_block(height + 1).await {
              Ok(block) => {
                if block.header.prev_blockhash != current {
                  ic_cdk::println!("reorg detected! our best = {}({})", height, current);
                  sync(60);
                  return;
                }
                ic_cdk::println!("indexing block {:?}", block.header);
                if let Err(e) = updater::index_block(height + 1, hash, block).await {
                  ic_cdk::println!("index error: {:?}", e);
                }
                sync(0);
              }
              Err(e) => {
                ic_cdk::println!("error: {:?}", e);
                sync(3);
              }
            }
          }
        }
        Err(e) => {
          ic_cdk::println!("error: {:?}", e);
          sync(3);
        }
      }
    });
  });
}
