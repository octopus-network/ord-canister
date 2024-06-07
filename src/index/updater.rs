mod rune_updater;

use self::rune_updater::RuneUpdater;
use crate::*;
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

// TODO handle best > height
pub(crate) async fn get_block(height: u32) -> Result<Option<BlockData>> {
  let hash = rpc::get_block_hash(height).await?;
  let block = rpc::get_block(hash).await?;
  Ok(Some(BlockData::from(block)))
}

pub(crate) async fn index_block(height: u32, block: BlockData) -> Result<()> {
  let mut updater = RuneUpdater {
    block_time: block.header.time,
    burned: HashMap::new(),
    // TODO
    event_handler: None,
    height,
    minimum: Rune::minimum_at_height(Network::Bitcoin, Height(height)),
  };

  for (i, (tx, txid)) in block.txdata.iter().enumerate() {
    updater.index_runes(u32::try_from(i).unwrap(), tx, *txid)?;
  }

  updater.update()?;
  Ok(())
}
