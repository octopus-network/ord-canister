use super::*;
use crate::index::entry::RuneBalance;

pub(super) struct RuneUpdater {
  pub(super) block_time: u32,
  pub(super) burned: HashMap<RuneId, Lot>,
  pub(super) height: u32,
  pub(super) minimum: Rune,
  pub(super) runes: u64,
  pub(super) change_record: ChangeRecord,
}

impl RuneUpdater {
  pub(super) async fn index_runes(
    &mut self,
    tx_index: u32,
    tx: &Transaction,
    txid: Txid,
  ) -> Result<()> {
    let artifact = Runestone::decipher(tx);

    let mut unallocated = self.unallocated(tx)?;

    let mut allocated: Vec<HashMap<RuneId, Lot>> = vec![HashMap::new(); tx.output.len()];

    if let Some(artifact) = &artifact {
      if let Some(id) = artifact.mint() {
        if let Some(amount) = self.mint(id)? {
          *unallocated.entry(id).or_default() += amount;

          // log!(
          //   INFO,
          //   "Rune minted: block_height: {}, txid: {:?}, rune_id: {:?}, amount: {:?}",
          //   self.height,
          //   txid,
          //   id,
          //   amount.n()
          // );
        }
      }

      let etched = self.etched(tx_index, tx, artifact).await?;

      if let Artifact::Runestone(runestone) = artifact {
        if let Some((id, ..)) = etched {
          *unallocated.entry(id).or_default() +=
            runestone.etching.unwrap().premine.unwrap_or_default();
        }

        for Edict { id, amount, output } in runestone.edicts.iter().copied() {
          let amount = Lot(amount);

          // edicts with output values greater than the number of outputs
          // should never be produced by the edict parser
          let output = usize::try_from(output).unwrap();
          assert!(output <= tx.output.len());

          let id = if id == RuneId::default() {
            let Some((id, ..)) = etched else {
              continue;
            };

            id
          } else {
            id
          };

          let Some(balance) = unallocated.get_mut(&id) else {
            continue;
          };

          let mut allocate = |balance: &mut Lot, amount: Lot, output: usize| {
            if amount > 0 {
              *balance -= amount;
              *allocated[output].entry(id).or_default() += amount;
            }
          };

          if output == tx.output.len() {
            // find non-OP_RETURN outputs
            let destinations = tx
              .output
              .iter()
              .enumerate()
              .filter_map(|(output, tx_out)| {
                (!tx_out.script_pubkey.is_op_return()).then_some(output)
              })
              .collect::<Vec<usize>>();

            if !destinations.is_empty() {
              if amount == 0 {
                // if amount is zero, divide balance between eligible outputs
                let amount = *balance / destinations.len() as u128;
                let remainder = usize::try_from(*balance % destinations.len() as u128).unwrap();

                for (i, output) in destinations.iter().enumerate() {
                  allocate(
                    balance,
                    if i < remainder { amount + 1 } else { amount },
                    *output,
                  );
                }
              } else {
                // if amount is non-zero, distribute amount to eligible outputs
                for output in destinations {
                  allocate(balance, amount.min(*balance), output);
                }
              }
            }
          } else {
            // Get the allocatable amount
            let amount = if amount == 0 {
              *balance
            } else {
              amount.min(*balance)
            };

            allocate(balance, amount, output);
          }
        }
      }

      if let Some((id, rune)) = etched {
        self.create_rune_entry(txid, artifact, id, rune)?;
      }
    }

    let mut burned: HashMap<RuneId, Lot> = HashMap::new();

    if let Some(Artifact::Cenotaph(_)) = artifact {
      for (id, balance) in unallocated {
        *burned.entry(id).or_default() += balance;
      }
    } else {
      let pointer = artifact
        .map(|artifact| match artifact {
          Artifact::Runestone(runestone) => runestone.pointer,
          Artifact::Cenotaph(_) => unreachable!(),
        })
        .unwrap_or_default();

      // assign all un-allocated runes to the default output, or the first non
      // OP_RETURN output if there is no default
      if let Some(vout) = pointer
        .map(|pointer| pointer.into_usize())
        .inspect(|&pointer| assert!(pointer < allocated.len()))
        .or_else(|| {
          tx.output
            .iter()
            .enumerate()
            .find(|(_vout, tx_out)| !tx_out.script_pubkey.is_op_return())
            .map(|(vout, _tx_out)| vout)
        })
      {
        for (id, balance) in unallocated {
          if balance > 0 {
            *allocated[vout].entry(id).or_default() += balance;
          }
        }
      } else {
        for (id, balance) in unallocated {
          if balance > 0 {
            *burned.entry(id).or_default() += balance;
          }
        }
      }
    }

    // update outpoint balances
    for (vout, balances) in allocated.into_iter().enumerate() {
      if balances.is_empty() {
        continue;
      }

      // increment burned balances
      if tx.output[vout].script_pubkey.is_op_return() {
        for (id, balance) in &balances {
          *burned.entry(*id).or_default() += *balance;
        }
        continue;
      }

      // let mut balances = balances.into_iter().collect::<Vec<(RuneId, Lot)>>();

      // Sort balances by id so tests can assert balances in a fixed order
      // balances.sort();

      let outpoint = OutPoint {
        txid,
        vout: vout.try_into().unwrap(),
      };

      let mut rune_balances = RuneBalances { balances: vec![] };

      for (id, balance) in balances {
        rune_balances.balances.push(RuneBalance {
          rune_id: id,
          balance: balance.n(),
        });

        // log!(INFO, "Rune transferred: outpoint: {:?}, block_height: {}, txid: {:?}, rune_id: {:?}, amount: {:?}", outpoint, self.height, txid, id, balance.n());
      }
      crate::index::mem_insert_outpoint_to_rune_balances(outpoint.store(), rune_balances);
      crate::index::mem_insert_outpoint_to_height(outpoint.store(), self.height);

      self.change_record.added_outpoints.push(outpoint);
    }

    // increment entries with burned runes
    for (id, amount) in burned {
      *self.burned.entry(id).or_default() += amount;

      log!(
        INFO,
        "Rune burned: block_height: {}, txid: {:?}, rune_id: {:?}, amount: {:?}",
        self.height,
        txid,
        id,
        amount.n()
      );
    }

    Ok(())
  }

  pub(super) fn update(mut self) -> Result {
    for (rune_id, burned) in self.burned {
      let mut entry = crate::index::mem_get_rune_id_to_rune_entry(rune_id.store()).unwrap();

      if !self.change_record.burned.contains_key(&rune_id) {
        self.change_record.burned.insert(rune_id, entry.burned);
      }

      entry.burned = entry.burned.checked_add(burned.n()).unwrap();
      crate::index::mem_insert_rune_id_to_rune_entry(rune_id.store(), entry);
    }

    crate::index::mem_insert_change_record(self.height, self.change_record);

    Ok(())
  }

  fn create_rune_entry(
    &mut self,
    txid: Txid,
    artifact: &Artifact,
    id: RuneId,
    rune: Rune,
  ) -> Result {
    crate::index::mem_insert_rune_to_rune_id(rune.store(), id.store());
    crate::index::mem_insert_transaction_id_to_rune(txid.store(), rune.store());

    let number = self.runes;
    self.runes += 1;

    crate::index::mem_insert_statistic_runes(self.height, self.runes);

    let entry = match artifact {
      Artifact::Cenotaph(_) => RuneEntry {
        block: id.block,
        burned: 0,
        divisibility: 0,
        etching: txid,
        terms: None,
        mints: 0,
        number,
        premine: 0,
        spaced_rune: SpacedRune { rune, spacers: 0 },
        symbol: None,
        timestamp: self.block_time.into(),
        turbo: false,
      },
      Artifact::Runestone(Runestone { etching, .. }) => {
        let Etching {
          divisibility,
          terms,
          premine,
          spacers,
          symbol,
          turbo,
          ..
        } = etching.unwrap();

        RuneEntry {
          block: id.block,
          burned: 0,
          divisibility: divisibility.unwrap_or_default(),
          etching: txid,
          terms,
          mints: 0,
          number,
          premine: premine.unwrap_or_default(),
          spaced_rune: SpacedRune {
            rune,
            spacers: spacers.unwrap_or_default(),
          },
          symbol,
          timestamp: self.block_time.into(),
          turbo,
        }
      }
    };

    crate::index::mem_insert_rune_id_to_rune_entry(id.store(), entry);

    self.change_record.added_runes.push((rune, id, txid));

    log!(
      INFO,
      "Rune etched: block_height: {}, txid: {:?}, rune_id: {:?}",
      self.height,
      txid,
      id
    );

    Ok(())
  }

  async fn etched(
    &mut self,
    tx_index: u32,
    tx: &Transaction,
    artifact: &Artifact,
  ) -> Result<Option<(RuneId, Rune)>> {
    let rune = match artifact {
      Artifact::Runestone(runestone) => match runestone.etching {
        Some(etching) => etching.rune,
        None => return Ok(None),
      },
      Artifact::Cenotaph(cenotaph) => match cenotaph.etching {
        Some(rune) => Some(rune),
        None => return Ok(None),
      },
    };

    let rune = if let Some(rune) = rune {
      if rune < self.minimum
        || rune.is_reserved()
        || crate::index::mem_get_rune_to_rune_id(rune.store()).is_some()
        || !self.tx_commits_to_rune(tx, rune).await?
      {
        return Ok(None);
      }
      rune
    } else {
      let reserved_runes = crate::index::mem_statistic_reserved_runes();

      crate::index::mem_insert_statistic_reserved_runes(self.height, reserved_runes + 1);

      Rune::reserved(self.height.into(), tx_index)
    };

    Ok(Some((
      RuneId {
        block: self.height.into(),
        tx: tx_index,
      },
      rune,
    )))
  }

  fn mint(&mut self, id: RuneId) -> Result<Option<Lot>> {
    let Some(mut rune_entry) = crate::index::mem_get_rune_id_to_rune_entry(id.store()) else {
      return Ok(None);
    };

    let Ok(amount) = rune_entry.mintable(self.height.into()) else {
      return Ok(None);
    };

    if !self.change_record.mints.contains_key(&id) {
      self.change_record.mints.insert(id, rune_entry.mints);
    }

    rune_entry.mints += 1;

    crate::index::mem_insert_rune_id_to_rune_entry(id.store(), rune_entry);

    Ok(Some(Lot(amount)))
  }

  async fn tx_commits_to_rune(&self, tx: &Transaction, rune: Rune) -> Result<bool> {
    let commitment = rune.commitment();

    for input in &tx.input {
      // extracting a tapscript does not indicate that the input being spent
      // was actually a taproot output. this is checked below, when we load the
      // output's entry from the database
      let Some(tapscript) = input.witness.tapscript() else {
        continue;
      };

      for instruction in tapscript.instructions() {
        // ignore errors, since the extracted script may not be valid
        let Ok(instruction) = instruction else {
          break;
        };

        let Some(pushbytes) = instruction.push_bytes() else {
          continue;
        };

        if pushbytes.as_bytes() != commitment {
          continue;
        }

        let tx_info =
          crate::rpc::get_raw_transaction_info(&input.previous_output.txid, None).await?;

        let taproot = tx_info.vout[input.previous_output.vout.into_usize()]
          .script_pub_key
          .script()?
          .is_p2tr();

        if !taproot {
          continue;
        }

        let commit_tx_height = crate::rpc::get_block_header_info(&tx_info.blockhash.unwrap())
          .await?
          .height;

        let confirmations = self
          .height
          .checked_sub(commit_tx_height.try_into().unwrap())
          .unwrap()
          + 1;

        if confirmations >= Runestone::COMMIT_CONFIRMATIONS as u32 {
          return Ok(true);
        }
      }
    }

    Ok(false)
  }

  fn unallocated(&mut self, tx: &Transaction) -> Result<HashMap<RuneId, Lot>> {
    // map of rune ID to un-allocated balance of that rune
    let mut unallocated: HashMap<RuneId, Lot> = HashMap::new();

    // increment unallocated runes with the runes in tx inputs
    for input in &tx.input {
      if let Some(rune_balances) =
        crate::index::mem_remove_outpoint_to_rune_balances(input.previous_output.store())
      {
        for rune_balance in rune_balances.balances.clone() {
          *unallocated.entry(rune_balance.rune_id).or_default() += rune_balance.balance;
        }
        let height = crate::index::mem_remove_outpoint_to_height(input.previous_output.store())
          .ok_or_else(|| {
            anyhow!(
              "Outpoint not found in outpoint_to_height: {:?}",
              input.previous_output
            )
          })?;

        self
          .change_record
          .removed_outpoints
          .push((input.previous_output, rune_balances, height));
      }
    }

    Ok(unallocated)
  }
}
