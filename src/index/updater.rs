mod rune_updater;

use self::rune_updater::RuneUpdater;
use crate::ic_log::*;
use crate::index::reorg::Reorg;
use crate::*;
use candid::Principal;
use ic_canister_log::log;
use ord_canister_interface::OrdError;
use std::collections::HashMap;

pub(crate) struct BlockData {
  pub(crate) header: Header,
  pub(crate) txdata: Vec<(Transaction, Txid)>,
}

impl From<Block> for BlockData {
  fn from(block: Block) -> Self {
    BlockData {
      header: block.header,
      txdata: block
        .txdata
        .into_iter()
        .map(|transaction| {
          let txid = transaction.txid();
          (transaction, txid)
        })
        .collect(),
    }
  }
}

pub(crate) async fn index_block(height: u32, block: BlockData) -> Result<()> {
  log!(
    INFO,
    "indexing block {:?} with block_hash: {:?}",
    height,
    block.header.block_hash()
  );
  let mut updater = RuneUpdater {
    block_time: block.header.time,
    burned: HashMap::new(),
    event_handler: None,
    height,
    minimum: Rune::minimum_at_height(Network::Bitcoin, Height(height)),
  };
  for (i, (tx, txid)) in block.txdata.iter().enumerate() {
    updater.index_runes(u32::try_from(i).unwrap(), tx, *txid)?;
  }
  updater.update()?;
  index::increase_height(height, block.header.block_hash());
  Ok(())
}

pub(crate) async fn get_block(hash: BlockHash) -> Result<BlockData> {
  let url = get_url();
  let block = rpc::get_block(&url, hash).await?;

  if block.block_hash() != hash {
    return Err(OrdError::WrongBlockHash(hash.to_string()));
  }

  block
    .check_merkle_root()
    .then(|| BlockData::from(block))
    .ok_or(OrdError::WrongBlockMerkleRoot(hash.to_string()))
}

pub fn update_index() {
  ic_cdk_timers::set_timer(std::time::Duration::from_secs(10), || {
    ic_cdk::spawn(async move {
      let (cur_height, _) = crate::highest_block();
      match crate::btc_canister::get_block_hash(cur_height + 1).await {
        Ok(Some(block_hash)) => {
          if let Ok(block) = get_block(block_hash).await {
            match Reorg::detect_reorg(block.header.prev_blockhash, cur_height + 1).await {
              Err(OrdError::Recoverable { height, depth }) => {
                Reorg::handle_reorg(height, depth);
                update_index();
              }
              Err(OrdError::Unrecoverable) => {
                log!(
                  CRITICAL,
                  "unrecoverable reorg detected at height {}",
                  cur_height + 1
                );
                return;
              }
              _ => {
                let block_hash = block.header.block_hash().to_string();
                let txids: Vec<String> = block
                  .txdata
                  .iter()
                  .map(|(_, txid)| txid.to_string())
                  .collect();
                if let Err(e) = index_block(cur_height + 1, block).await {
                  log!(CRITICAL, "failed to index_block: {:?}", e);
                } else {
                  let subscribers = crate::memory::get_subscribers();
                  for subscriber in subscribers
                    .iter()
                    .filter_map(|s| Principal::from_text(s).ok())
                  {
                    let _ = crate::notifier::notify_new_block(
                      subscriber,
                      cur_height + 1,
                      block_hash.clone(),
                      txids.clone(),
                    )
                    .await;
                    log!(
                      INFO,
                      "notified subscriber: {:?} with block_height: {:?} block_hash: {:?}",
                      subscriber,
                      cur_height + 1,
                      block_hash
                    );
                  }
                  update_index();
                }
              }
            }
          } else {
            log!(CRITICAL, "failed to get_block: {:?}", block_hash);
            update_index();
          }
        }
        Err(e) => {
          log!(CRITICAL, "failed to get_block_hash: {:?}", e);
          update_index();
        }
        _ => {
          update_index();
        }
      }
    });
  });
}
