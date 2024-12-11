mod rune_updater;

use self::rune_updater::RuneUpdater;
use crate::ic_log::*;
use crate::index::reorg::Reorg;
use crate::*;
use ic_canister_log::log;
use rune_indexer_interface::OrdError;
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
  ic_cdk_timers::set_timer_interval(std::time::Duration::from_secs(10), || {
    ic_cdk::spawn(async move {
      let (cur_height, _) = crate::highest_block();
      if let Ok(Some(block_hash)) = crate::btc_canister::get_block_hash(cur_height + 1).await {
        if let Ok(block) = get_block(block_hash).await {
          match Reorg::detect_reorg(block.header.prev_blockhash, cur_height + 1).await {
            Err(OrdError::Recoverable { height, depth }) => {
              Reorg::handle_reorg(height, depth);
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
              if let Err(e) = index_block(cur_height + 1, block).await {
                log!(CRITICAL, "failed to index_block: {:?}", e);
              }
            }
          }
        } else {
          log!(CRITICAL, "failed to get_block");
        }
      } else {
        log!(CRITICAL, "failed to get_block_hash");
      }
    });
  });
}
