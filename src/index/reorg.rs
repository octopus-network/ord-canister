use crate::index::INFO;
use crate::Result;
use bitcoin::block::BlockHash;
use ic_canister_log::log;
use ord_canister_interface::OrdError;

const MAX_RECOVERABLE_REORG_DEPTH: u32 = 6;

pub(crate) struct Reorg {}

impl Reorg {
  pub(crate) async fn detect_reorg(bitcoind_prev_blockhash: BlockHash, height: u32) -> Result<()> {
    match crate::block_hash(height.checked_sub(1).expect("height overflow")) {
      Some(index_prev_blockhash) if index_prev_blockhash == bitcoind_prev_blockhash => Ok(()),
      Some(index_prev_blockhash) if index_prev_blockhash != bitcoind_prev_blockhash => {
        for depth in 2..MAX_RECOVERABLE_REORG_DEPTH {
          let index_block_hash =
            crate::block_hash(height.checked_sub(depth).expect("height overflow"))
              .ok_or(OrdError::Unrecoverable)?;

          let bitcoin_canister_block_hash = crate::btc_canister::get_block_hash(
            height.checked_sub(depth).expect("height overflow"),
          )
          .await?
          .ok_or(OrdError::Unrecoverable)?;

          if index_block_hash == bitcoin_canister_block_hash {
            return Err(OrdError::Recoverable { height, depth });
          }
        }

        Err(OrdError::Unrecoverable)
      }
      _ => Ok(()),
    }
  }

  pub(crate) fn handle_reorg(height: u32, depth: u32) {
    log!(
      INFO,
      "rolling back database after reorg of depth {depth} at height {height}"
    );

    for h in (height - depth + 1..height).rev() {
      crate::height_to_outpoints(|o| match o.get(&h) {
        Some(outpoints) => {
          outpoints.iter().for_each(|outpoint| {
            crate::outpoint_to_rune_balances(|balances| balances.remove(&outpoint));
            crate::outpoint_to_height(|o| o.remove(&outpoint));
          });
        }
        None => {}
      });
      crate::height_to_outpoints(|o| o.remove(&h));

      crate::height_to_rune_ids(|htri| match htri.get(&h) {
        Some(rune_ids) => {
          rune_ids.iter().for_each(|rune_id| {
            crate::rune_id_to_rune_entry(|rune_entry| rune_entry.remove(&rune_id));
          });
        }
        None => {}
      });
      crate::height_to_rune_ids(|htri| htri.remove(&h));

      crate::height_to_rune_updates(|htru| htru.remove(&h));

      crate::height_to_block_hash(|htbh| htbh.remove(&h));
    }

    crate::height_to_rune_updates(|htru| match htru.get(&(height - depth)) {
      Some(rune_updates) => {
        rune_updates.iter().for_each(|rune_update| {
          crate::rune_id_to_rune_entry(|ritre| {
            if let Some(mut rune_entry) = ritre.get_mut(&rune_update.id) {
              rune_entry.mints = rune_update.mints;
              rune_entry.burned = rune_update.burned;
            }
            true
          });
        });
      }
      None => {}
    });

    log!(
      INFO,
      "successfully rolled back database to height {}",
      height - depth,
    );
  }
}
