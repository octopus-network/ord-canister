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
            if let Err(e) = index_block(height, block).await {
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
        Ok(None) => {}
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

async fn index_block(height: u32, block: BlockData) -> Result<()> {
  // Reorg::detect_reorg(&block, self.height, self.index)?;

  log!(
    INFO,
    "Block {} at {} with {} transactions…",
    height,
    timestamp(block.header.time.into()),
    block.txdata.len()
  );

  // Log statistics every 200 blocks
  if height % 200 == 0 {
    log!(
      INFO,
      "Index statistics at height {}: latest_block: {:?}, reserved_runes: {}, runes: {}, rune_to_rune_id: {}, rune_entry: {}, transaction_id_to_rune: {}, rune_balance: {}, outpoint_to_height: {}",
      height,
      crate::index::mem_latest_block(),
      crate::index::mem_statistic_reserved_runes(),
      crate::index::mem_statistic_runes(),
      crate::index::mem_length_rune_to_rune_id(),
      crate::index::mem_length_rune_entry(),
      crate::index::mem_length_transaction_id_to_rune(),
      crate::index::mem_length_rune_balance(),
      crate::index::mem_length_outpoint_to_height()
    );
  }

  let runes = crate::index::mem_statistic_runes();

  let mut rune_updater = RuneUpdater {
    block_time: block.header.time,
    burned: HashMap::new(),
    height,
    minimum: Rune::minimum_at_height(bitcoin::Network::Bitcoin, Height(height)),
    runes,
  };

  for (i, (tx, txid)) in block.txdata.iter().enumerate() {
    rune_updater
      .index_runes(u32::try_from(i).unwrap(), tx, *txid)
      .await?;
  }

  rune_updater.update()?;

  crate::index::mem_insert_block_header(height, block.header.store());

  Ok(())
}
