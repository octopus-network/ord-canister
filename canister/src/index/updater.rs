use self::rune_updater::RuneUpdater;
use super::*;

mod rune_updater;

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
          let txid = transaction.compute_txid();
          (transaction, txid)
        })
        .collect(),
    }
  }
}

pub fn update_index(network: BitcoinNetwork) -> Result {
  ic_cdk_timers::set_timer(std::time::Duration::from_secs(10), move || {
    ic_cdk::spawn(async move {
      let (height, prev_blockhash) = crate::index::next_block(network);
      match crate::bitcoin_api::get_block_hash(height).await {
        Ok(Some(block_hash)) => match crate::rpc::get_block(block_hash).await {
          Ok(block) => {
            if let Err(e) = index_block(height, block) {
              log!(
                CRITICAL,
                "failed to index_block at height {}: {:?}",
                height,
                e
              );
              return;
            }
          }
          Err(e) => {
            log!(
              CRITICAL,
              "failed to get_block: {:?} error: {:?}",
              block_hash,
              e
            );
          }
        },
        Ok(None) => {
          log!(CRITICAL, "failed to get_block_hash at height {}", height);
        }
        Err(e) => {
          log!(
            CRITICAL,
            "failed to get_block_hash at height {}: {:?}",
            height,
            e
          );
        }
      }
      update_index(network);
    });
  });

  Ok(())
}

fn index_block(height: u32, block: BlockData) -> Result<()> {
  // Reorg::detect_reorg(&block, self.height, self.index)?;

  log!(
    INFO,
    "Block {} at {} with {} transactions…",
    height,
    timestamp(block.header.time.into()),
    block.txdata.len()
  );

  let runes = crate::index::mem_statistic_runes();

  let mut rune_updater = RuneUpdater {
    block_time: block.header.time,
    burned: HashMap::new(),
    height,
    minimum: Rune::minimum_at_height(bitcoin::Network::Bitcoin, Height(height)),
    runes,
  };

  for (i, (tx, txid)) in block.txdata.iter().enumerate() {
    rune_updater.index_runes(u32::try_from(i).unwrap(), tx, *txid)?;
  }

  rune_updater.update()?;

  crate::index::mem_insert_block_header(height, block.header.store());

  Ok(())
}
