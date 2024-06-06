use {
  self::{
    entry::{
      Entry, HeaderValue, OutPointValue, RuneEntryValue, RuneIdValue, SatPointValue, SatRange,
      TxOutValue, TxidValue,
    },
    event::Event,
    lot::Lot,
  },
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

pub(crate) fn encode_rune_balance(id: RuneId, balance: u128, buffer: &mut Vec<u8>) {
  varint::encode_to_vec(id.block.into(), buffer);
  varint::encode_to_vec(id.tx.into(), buffer);
  varint::encode_to_vec(balance, buffer);
}

pub(crate) fn get_etching(txid: Txid) -> Result<Option<SpacedRune>> {
  let Some(rune) = crate::transaction_id_to_rune(|t| t.get(&Txid::store(txid)).map(|r| *r)) else {
    return Ok(None);
  };

  let id = crate::rune_to_rune_id(|r| *r.get(&rune).unwrap());

  let entry = crate::rune_id_to_rune_entry(|r| *r.get(&id).unwrap());

  Ok(Some(entry.spaced_rune))
}

pub(crate) fn decode_rune_balance(buffer: &[u8]) -> Result<((RuneId, u128), usize)> {
  let mut len = 0;
  let (block, block_len) = varint::decode(&buffer[len..])?;
  len += block_len;
  let (tx, tx_len) = varint::decode(&buffer[len..])?;
  len += tx_len;
  let id = RuneId {
    block: block.try_into()?,
    tx: tx.try_into()?,
  };
  let (balance, balance_len) = varint::decode(&buffer[len..])?;
  len += balance_len;
  Ok(((id, balance), len))
}

pub(crate) fn get_rune_balances_for_output(
  outpoint: OutPoint,
) -> Result<BTreeMap<SpacedRune, Pile>> {
  // let outpoint_to_balances = rtx.open_table(OUTPOINT_TO_RUNE_BALANCES)?;

  // let id_to_rune_entries = rtx.open_table(RUNE_ID_TO_RUNE_ENTRY)?;

  let Some(balances) =
    crate::outpoint_to_rune_balances(|o| o.get(&OutPoint::store(outpoint)).map(|b| *b))
  else {
    return Ok(BTreeMap::new());
  };

  // let balances_buffer = balances.value();

  let mut result = BTreeMap::new();
  for rune in balances.iter() {
    let rune = *rune;
    // let ((id, amount), length) = decode_rune_balance(&balances_buffer[i..]).unwrap();

    let entry = rune_id_to_rune_entry(|r| r.get(id.store()).map(|r| *r).unwrap());

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

// pub(crate) trait BitcoinCoreRpcResultExt<T> {
//   fn into_option(self) -> Result<Option<T>>;
// }

// impl<T> BitcoinCoreRpcResultExt<T> for Result<T, bitcoincore_rpc::Error> {
//   fn into_option(self) -> Result<Option<T>> {
//     match self {
//       Ok(ok) => Ok(Some(ok)),
//       Err(bitcoincore_rpc::Error::JsonRpc(bitcoincore_rpc::jsonrpc::error::Error::Rpc(
//         bitcoincore_rpc::jsonrpc::error::RpcError { code: -8, .. },
//       ))) => Ok(None),
//       Err(bitcoincore_rpc::Error::JsonRpc(bitcoincore_rpc::jsonrpc::error::Error::Rpc(
//         bitcoincore_rpc::jsonrpc::error::RpcError { message, .. },
//       )))
//         if message.ends_with("not found") =>
//       {
//         Ok(None)
//       }
//       Err(err) => Err(err.into()),
//     }
//   }
// }

// pub struct Index {
// pub(crate) client: Client,
// database: Database,
// durability: redb::Durability,
// TODO Box<Fn<Event>>
// event_sender: Option<tokio::sync::mpsc::Sender<Event>>,
// first_inscription_height: u32,
// genesis_block_coinbase_transaction: Transaction,
// genesis_block_coinbase_txid: Txid,
// height_limit: Option<u32>,
// index_addresses: bool,
// index_runes: bool,
// index_sats: bool,
// index_spent_sats: bool,
// index_transactions: bool,
// path: PathBuf,
// settings: Settings,
// started: DateTime<Utc>,
// unrecoverably_reorged: AtomicBool,
// }

// impl Index {
// pub fn open(settings: &Settings) -> Result<Self> {
//   Index::open_with_event_sender(settings, None)
// }

// pub fn open_with_event_sender(
//   settings: &Settings,
//   event_sender: Option<tokio::sync::mpsc::Sender<Event>>,
// ) -> Result<Self> {
//   let database = match Database::builder()
//     .set_cache_size(index_cache_size)
//     .set_repair_callback(repair_callback)
//     .open(&path)
//   {
//     Ok(database) => database,
//     Err(DatabaseError::Storage(StorageError::Io(error)))
//       if error.kind() == io::ErrorKind::NotFound =>
//     {
//       let database = Database::builder()
//         .set_cache_size(index_cache_size)
//         .create(&path)?;

//       let mut tx = database.begin_write()?;

//       // seems like init
//       if settings.index_runes() && settings.chain() == Chain::Mainnet {
//         let rune = Rune(2055900680524219742);

//         let id = RuneId { block: 1, tx: 0 };
//         let etching = Txid::all_zeros();

//         tx.open_table(RUNE_TO_RUNE_ID)?
//           .insert(rune.store(), id.store())?;

//         tx.open_table(RUNE_ID_TO_RUNE_ENTRY)?.insert(
//           id.store(),
//           RuneEntry {
//             block: id.block,
//             burned: 0,
//             divisibility: 0,
//             etching,
//             terms: Some(Terms {
//               amount: Some(1),
//               cap: Some(u128::MAX),
//               height: (
//                 Some((SUBSIDY_HALVING_INTERVAL * 4).into()),
//                 Some((SUBSIDY_HALVING_INTERVAL * 5).into()),
//               ),
//               offset: (None, None),
//             }),
//             mints: 0,
//             number: 0,
//             premine: 0,
//             spaced_rune: SpacedRune { rune, spacers: 128 },
//             symbol: Some('\u{29C9}'),
//             timestamp: 0,
//             turbo: true,
//           }
//           .store(),
//         )?;

//         tx.open_table(TRANSACTION_ID_TO_RUNE)?
//           .insert(&etching.store(), rune.store())?;
//       }

//       tx.commit()?;

//       database
//     }
//     Err(error) => bail!("failed to open index: {error}"),
//   };

//   let index_addresses;
//   let index_runes;
//   let index_sats;
//   let index_spent_sats;
//   let index_transactions;

//   {
//     let tx = database.begin_read()?;
//     let statistics = tx.open_table(STATISTIC_TO_COUNT)?;
//     index_addresses = Self::is_statistic_set(&statistics, Statistic::IndexAddresses)?;
//     index_runes = Self::is_statistic_set(&statistics, Statistic::IndexRunes)?;
//     index_sats = Self::is_statistic_set(&statistics, Statistic::IndexSats)?;
//     index_spent_sats = Self::is_statistic_set(&statistics, Statistic::IndexSpentSats)?;
//     index_transactions = Self::is_statistic_set(&statistics, Statistic::IndexTransactions)?;
//   }

//   let genesis_block_coinbase_transaction =
//     settings.chain().genesis_block().coinbase().unwrap().clone();

//   Ok(Self {
//     genesis_block_coinbase_txid: genesis_block_coinbase_transaction.txid(),
//     client,
//     database,
//     durability,
//     event_sender,
//     first_inscription_height: settings.first_inscription_height(),
//     genesis_block_coinbase_transaction,
//     height_limit: settings.height_limit(),
//     index_addresses,
//     index_runes,
//     index_sats,
//     index_spent_sats,
//     index_transactions,
//     settings: settings.clone(),
//     path,
//     started: Utc::now(),
//     unrecoverably_reorged: AtomicBool::new(false),
//   })
// }

// pub fn update() -> Result {
//   loop {
//     let wtx = self.begin_write()?;

//     let mut updater = Updater {
//       height: wtx
//         .open_table(HEIGHT_TO_BLOCK_HEADER)?
//         .range(0..)?
//         .next_back()
//         .transpose()?
//         .map(|(height, _header)| height.value() + 1)
//         .unwrap_or(0),
//       index: self,
//       outputs_cached: 0,
//       outputs_inserted_since_flush: 0,
//       outputs_traversed: 0,
//       range_cache: HashMap::new(),
//       sat_ranges_since_flush: 0,
//     };

//     match updater.update_index(wtx) {
//       Ok(ok) => return Ok(ok),
//       Err(err) => {
//         match err.downcast_ref() {
//           Some(&reorg::Error::Recoverable { height, depth }) => {
//             Reorg::handle_reorg(self, height, depth)?;
//           }
//           Some(&reorg::Error::Unrecoverable) => {
//             self
//               .unrecoverably_reorged
//               .store(true, atomic::Ordering::Relaxed);
//             return Err(anyhow!(reorg::Error::Unrecoverable));
//           }
//           _ => return Err(err),
//         };
//       }
//     }
//   }
// }

// pub(crate) fn block_count(&self) -> Result<u32> {
//   self.begin_read()?.block_count()
// }

// pub(crate) fn block_height(&self) -> Result<Option<Height>> {
//   self.begin_read()?.block_height()
// }

// pub(crate) fn block_hash(&self, height: Option<u32>) -> Result<Option<BlockHash>> {
//   self.begin_read()?.block_hash(height)
// }

// pub(crate) fn blocks(&self, take: usize) -> Result<Vec<(u32, BlockHash)>> {
//   let rtx = self.begin_read()?;

//   let block_count = rtx.block_count()?;

//   let height_to_block_header = rtx.0.open_table(HEIGHT_TO_BLOCK_HEADER)?;

//   let mut blocks = Vec::with_capacity(block_count.try_into().unwrap());

//   for next in height_to_block_header
//     .range(0..block_count)?
//     .rev()
//     .take(take)
//   {
//     let next = next?;
//     blocks.push((next.0.value(), Header::load(*next.1.value()).block_hash()));
//   }

//   Ok(blocks)
// }

// pub(crate) fn get_rune_by_id(&self, id: RuneId) -> Result<Option<Rune>> {
//   Ok(crate::rune_id_to_entry(|r| {
//     r.get(&id)
//       .map(|entry| RuneEntry::load(entry.value()).spaced_rune.rune)
//   }))
// }

// pub(crate) fn get_rune_by_number(&self, number: usize) -> Result<Option<Rune>> {
//   crate::rune_id_to_entry(|r| match r.iter().nth(number) {
//     Some(result) => {
//       let rune_result =
//         result.map(|(_id, entry)| RuneEntry::load(entry.value()).spaced_rune.rune);
//       Ok(rune_result.ok())
//     }
//     None => Ok(None),
//   })
// }

// pub(crate) fn rune(
//   &self,
//   rune: Rune,
// ) -> Result<Option<(RuneId, RuneEntry, Option<InscriptionId>)>> {
//   let rtx = self.database.begin_read()?;

//   let Some(id) = rtx
//     .open_table(RUNE_TO_RUNE_ID)?
//     .get(rune.0)?
//     .map(|guard| guard.value())
//   else {
//     return Ok(None);
//   };

//   let entry = RuneEntry::load(
//     rtx
//       .open_table(RUNE_ID_TO_RUNE_ENTRY)?
//       .get(id)?
//       .unwrap()
//       .value(),
//   );

//   let parent = InscriptionId {
//     txid: entry.etching,
//     index: 0,
//   };

//   let parent = rtx
//     .open_table(INSCRIPTION_ID_TO_SEQUENCE_NUMBER)?
//     .get(&parent.store())?
//     .is_some()
//     .then_some(parent);

//   Ok(Some((RuneId::load(id), entry, parent)))
// }

// pub(crate) fn runes(&self) -> Result<Vec<(RuneId, RuneEntry)>> {
//   let mut entries = Vec::new();

//   crate::rune_id_to_entry(|r| {
//     for (id, entry) in r.iter() {
//       entries.push((RuneId::load(id.value()), RuneEntry::load(entry.value())));
//     }

//     Ok(())
//   })?;
//   Ok(entries)
// }

// pub(crate) fn runes_paginated(
//   &self,
//   page_size: usize,
//   page_index: usize,
// ) -> Result<(Vec<(RuneId, RuneEntry)>, bool)> {
//   let mut entries = Vec::new();

//   for result in self
//     .database
//     .begin_read()?
//     .open_table(RUNE_ID_TO_RUNE_ENTRY)?
//     .iter()?
//     .rev()
//     .skip(page_index.saturating_mul(page_size))
//     .take(page_size.saturating_add(1))
//   {
//     let (id, entry) = result?;
//     entries.push((RuneId::load(id.value()), RuneEntry::load(entry.value())));
//   }

//   let more = entries.len() > page_size;

//   Ok((entries, more))
// }

// pub(crate) fn get_rune_balance_map(
//   &self,
// ) -> Result<BTreeMap<SpacedRune, BTreeMap<OutPoint, Pile>>> {
//   let outpoint_balances = self.get_rune_balances()?;

//   let rtx = self.database.begin_read()?;

//   let rune_id_to_rune_entry = rtx.open_table(RUNE_ID_TO_RUNE_ENTRY)?;

//   let mut rune_balances_by_id: BTreeMap<RuneId, BTreeMap<OutPoint, u128>> = BTreeMap::new();

//   for (outpoint, balances) in outpoint_balances {
//     for (rune_id, amount) in balances {
//       *rune_balances_by_id
//         .entry(rune_id)
//         .or_default()
//         .entry(outpoint)
//         .or_default() += amount;
//     }
//   }

//   let mut rune_balances = BTreeMap::new();

//   for (rune_id, balances) in rune_balances_by_id {
//     let RuneEntry {
//       divisibility,
//       spaced_rune,
//       symbol,
//       ..
//     } = RuneEntry::load(
//       rune_id_to_rune_entry
//         .get(&rune_id.store())?
//         .unwrap()
//         .value(),
//     );

//     rune_balances.insert(
//       spaced_rune,
//       balances
//         .into_iter()
//         .map(|(outpoint, amount)| {
//           (
//             outpoint,
//             Pile {
//               amount,
//               divisibility,
//               symbol,
//             },
//           )
//         })
//         .collect(),
//     );
//   }

//   Ok(rune_balances)
// }

// pub(crate) fn get_rune_balances(&self) -> Result<Vec<(OutPoint, Vec<(RuneId, u128)>)>> {
//   let mut result = Vec::new();

//   for entry in self
//     .database
//     .begin_read()?
//     .open_table(OUTPOINT_TO_RUNE_BALANCES)?
//     .iter()?
//   {
//     let (outpoint, balances_buffer) = entry?;
//     let outpoint = OutPoint::load(*outpoint.value());
//     let balances_buffer = balances_buffer.value();

//     let mut balances = Vec::new();
//     let mut i = 0;
//     while i < balances_buffer.len() {
//       let ((id, balance), length) = Index::decode_rune_balance(&balances_buffer[i..]).unwrap();
//       i += length;
//       balances.push((id, balance));
//     }

//     result.push((outpoint, balances));
//   }

//   Ok(result)
// }

// pub(crate) fn block_header(&self, hash: BlockHash) -> Result<Option<Header>> {
//   self.client.get_block_header(&hash).into_option()
// }

// pub(crate) fn block_header_info(&self, hash: BlockHash) -> Result<Option<GetBlockHeaderResult>> {
//   self.client.get_block_header_info(&hash).into_option()
// }

// pub(crate) fn block_stats(&self, height: u64) -> Result<Option<GetBlockStatsResult>> {
//   self.client.get_block_stats(height).into_option()
// }

// pub(crate) fn is_output_spent(&self, outpoint: OutPoint) -> Result<bool> {
//   Ok(
//     outpoint != OutPoint::null()
//       && outpoint != self.settings.chain().genesis_coinbase_outpoint()
//       && if self.settings.index_addresses() {
//         self
//           .database
//           .begin_read()?
//           .open_table(OUTPOINT_TO_TXOUT)?
//           .get(&outpoint.store())?
//           .is_none()
//       } else {
//         self
//           .client
//           .get_tx_out(&outpoint.txid, outpoint.vout, Some(true))?
//           .is_none()
//       },
//   )
// }

// pub(crate) fn is_output_in_active_chain(&self, outpoint: OutPoint) -> Result<bool> {
//   if outpoint == OutPoint::null() {
//     return Ok(true);
//   }

//   if outpoint == self.settings.chain().genesis_coinbase_outpoint() {
//     return Ok(true);
//   }

//   let Some(info) = self
//     .client
//     .get_raw_transaction_info(&outpoint.txid, None)
//     .into_option()?
//   else {
//     return Ok(false);
//   };

//   if info.blockhash.is_none() {
//     return Ok(false);
//   }

//   if outpoint.vout.into_usize() >= info.vout.len() {
//     return Ok(false);
//   }

//   Ok(true)
// }

// pub(crate) fn get_runes_in_block(&self, block_height: u64) -> Result<Vec<SpacedRune>> {
//   let rtx = self.database.begin_read()?;

//   let rune_id_to_rune_entry = rtx.open_table(RUNE_ID_TO_RUNE_ENTRY)?;

//   let min_id = RuneId {
//     block: block_height,
//     tx: 0,
//   };

//   let max_id = RuneId {
//     block: block_height,
//     tx: u32::MAX,
//   };

//   let runes = rune_id_to_rune_entry
//     .range(min_id.store()..=max_id.store())?
//     .map(|result| result.map(|(_, entry)| RuneEntry::load(entry.value()).spaced_rune))
//     .collect::<Result<Vec<SpacedRune>, StorageError>>()?;

//   Ok(runes)
// }
// }
