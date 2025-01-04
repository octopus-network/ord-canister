use {
  self::rune_updater::RuneUpdater,
  super::{fetcher::Fetcher, *},
  futures::future::try_join_all,
  tokio::sync::{
    broadcast::{self, error::TryRecvError},
    mpsc::{self},
  },
};

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

pub(crate) struct Updater<'index> {
  pub(super) height: u32,
  pub(super) index: &'index Index,
}

impl Updater<'_> {
  pub(crate) fn update_index(&mut self, mut wtx: WriteTransaction) -> Result {
    let rx = Self::fetch_blocks_from(self.index, self.height)?;

    while let Ok(block) = rx.recv() {
      self.index_block(&mut wtx, block)?;
    }

    Ok(())
  }

  fn index_block(&mut self, wtx: &mut WriteTransaction, block: BlockData) -> Result<()> {
    Reorg::detect_reorg(&block, self.height, self.index)?;

    log::info!(
      "Block {} at {} with {} transactions…",
      self.height,
      timestamp(block.header.time.into()),
      block.txdata.len()
    );

    let mut height_to_block_header = wtx.open_table(HEIGHT_TO_BLOCK_HEADER)?;
    let mut statistic_to_count = wtx.open_table(STATISTIC_TO_COUNT)?;

    if self.index.index_runes && self.height >= self.index.settings.first_rune_height() {
      let mut outpoint_to_rune_balances = wtx.open_table(OUTPOINT_TO_RUNE_BALANCES)?;
      let mut rune_id_to_rune_entry = wtx.open_table(RUNE_ID_TO_RUNE_ENTRY)?;
      let mut rune_to_rune_id = wtx.open_table(RUNE_TO_RUNE_ID)?;
      let mut transaction_id_to_rune = wtx.open_table(TRANSACTION_ID_TO_RUNE)?;

      let runes = statistic_to_count
        .get(&Statistic::Runes.into())?
        .map(|x| x.value())
        .unwrap_or(0);

      let mut rune_updater = RuneUpdater {
        block_time: block.header.time,
        burned: HashMap::new(),
        height: self.height,
        id_to_entry: &mut rune_id_to_rune_entry,
        minimum: Rune::minimum_at_height(
          self.index.settings.chain().network(),
          Height(self.height),
        ),
        outpoint_to_balances: &mut outpoint_to_rune_balances,
        rune_to_id: &mut rune_to_rune_id,
        runes,
        statistic_to_count: &mut statistic_to_count,
        transaction_id_to_rune: &mut transaction_id_to_rune,
      };

      for (i, (tx, txid)) in block.txdata.iter().enumerate() {
        rune_updater.index_runes(u32::try_from(i).unwrap(), tx, *txid)?;
      }

      rune_updater.update()?;
    }

    height_to_block_header.insert(&self.height, &block.header.store())?;

    self.height += 1;

    Ok(())
  }
}
