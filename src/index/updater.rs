mod rune_updater;

use self::rune_updater::RuneUpdater;
use crate::index::reorg::Reorg;
use crate::*;
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
  Reorg::detect_reorg(&block, height).await?;
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

pub(crate) async fn get_block(height: u32) -> Result<BlockData> {
  let url = get_url();
  let hash = rpc::get_block_hash(&url, height).await?;
  let block = rpc::get_block(&url, hash).await?;
  block
    .check_merkle_root()
    .then(|| BlockData::from(block))
    .ok_or(OrdError::BlockVerification(height))
}
