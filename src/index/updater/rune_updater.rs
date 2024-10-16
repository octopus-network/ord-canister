use crate::index::{entry::RuneBalance, *};
use ic_stable_memory::collections::SVec;
use std::collections::HashMap;

pub(super) struct RuneUpdater {
  pub(super) block_time: u32,
  pub(super) burned: HashMap<RuneId, Lot>,
  pub(super) event_handler: Option<Box<dyn Fn(Event)>>,
  pub(super) height: u32,
  pub(super) minimum: Rune,
}

impl RuneUpdater {
  pub(super) fn index_runes(&mut self, tx_index: u32, tx: &Transaction, txid: Txid) -> Result<()> {
    let artifact = Runestone::decipher(tx);

    let mut unallocated = self.unallocated(tx)?;

    let mut allocated: Vec<HashMap<RuneId, Lot>> = vec![HashMap::new(); tx.output.len()];

    if let Some(artifact) = &artifact {
      if let Some(id) = artifact.mint() {
        if let Some(amount) = self.mint(id)? {
          *unallocated.entry(id).or_default() += amount;

          if let Some(handler) = &self.event_handler {
            handler(Event::RuneMinted {
              block_height: self.height,
              txid,
              rune_id: id,
              amount: amount.n(),
            });
          }
        }
      }

      let etched = self.etched(tx_index, tx, artifact)?;

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
        .map(|pointer| pointer as usize)
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
      let mut vec = SVec::new_with_capacity(balances.len()).expect("out of memory");
      for (id, balance) in balances {
        vec
          .push(RuneBalance {
            id,
            balance: balance.0,
          })
          .expect("MemoryOverflow");
        if let Some(handler) = &self.event_handler {
          handler(Event::RuneTransferred {
            outpoint,
            block_height: self.height,
            txid,
            rune_id: id,
            amount: balance.0,
          });
        }
      }

      outpoint_to_rune_balances(|b| b.insert(outpoint.store(), vec).expect("MemoryOverflow"));
    }

    // increment entries with burned runes
    for (id, amount) in burned {
      *self.burned.entry(id).or_default() += amount;

      if let Some(handler) = &self.event_handler {
        handler(Event::RuneBurned {
          block_height: self.height,
          txid,
          rune_id: id,
          amount: amount.n(),
        });
      }
    }

    Ok(())
  }

  pub(super) fn update(self) -> Result<()> {
    for (rune_id, burned) in self.burned {
      let mut entry = crate::rune_id_to_rune_entry(|r| *r.get(&rune_id).unwrap());
      entry.burned = entry.burned.checked_add(burned.n()).unwrap();
      crate::rune_id_to_rune_entry(|r| r.insert(rune_id, entry)).expect("MemoryOverflow");
    }

    Ok(())
  }

  fn create_rune_entry(
    &mut self,
    txid: Txid,
    artifact: &Artifact,
    id: RuneId,
    rune: Rune,
  ) -> Result<()> {
    // crate::rune_to_rune_id(|r| r.insert(rune.store(), id)).expect("MemoryOverflow");
    crate::transaction_id_to_rune(|t| t.insert(txid.store(), rune.0)).expect("MemoryOverflow");

    let entry = match artifact {
      Artifact::Cenotaph(_) => RuneEntry {
        block: id.block,
        burned: 0,
        divisibility: 0,
        etching: txid,
        terms: None,
        mints: 0,
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

    crate::rune_id_to_rune_entry(|r| r.insert(id, entry)).expect("Overflow");

    match &self.event_handler {
      Some(handler) => handler(Event::RuneEtched {
        block_height: self.height,
        txid,
        rune_id: id,
      }),
      None => {}
    }
    Ok(())
  }

  fn etched(
    &mut self,
    tx_index: u32,
    _tx: &Transaction,
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
      if rune < self.minimum || rune.is_reserved()
      // || crate::rune_to_rune_id(|r| r.get(&rune.0).is_some())
      // || !Self::tx_commits_to_rune(tx, rune).await?
      {
        return Ok(None);
      }
      rune
    } else {
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
    let Some(mut rune_entry) = crate::rune_id_to_rune_entry(|r| r.get(&id).map(|e| *e)) else {
      return Ok(None);
    };

    let Ok(amount) = rune_entry.mintable(self.height.into()) else {
      return Ok(None);
    };

    rune_entry.mints += 1;

    crate::rune_id_to_rune_entry(|r| r.insert(id, rune_entry)).expect("MemoryOverflow");

    Ok(Some(Lot(amount)))
  }

  // #[allow(dead_code)]
  // async fn tx_commits_to_rune(tx: &Transaction, rune: Rune) -> Result<bool> {
  //   let commitment = rune.commitment();

  //   for input in &tx.input {
  //     // extracting a tapscript does not indicate that the input being spent
  //     // was actually a taproot output. this is checked below, when we load the
  //     // output's entry from the database
  //     let Some(tapscript) = input.witness.tapscript() else {
  //       continue;
  //     };

  //     for instruction in tapscript.instructions() {
  //       // ignore errors, since the extracted script may not be valid
  //       let Ok(instruction) = instruction else {
  //         break;
  //       };

  //       let Some(pushbytes) = instruction.push_bytes() else {
  //         continue;
  //       };

  //       if pushbytes.as_bytes() != commitment {
  //         continue;
  //       }

  //       let tx_info = super::get_raw_tx(input.previous_output.txid).await?;

  //       let taproot = tx_info.vout[input.previous_output.vout as usize]
  //         .script_pub_key
  //         .script()
  //         .map_err(|e| OrdError::Params(e.to_string()))?
  //         .is_p2tr();

  //       if !taproot {
  //         continue;
  //       }
  //       return Ok(true);
  //     }
  //   }

  //   Ok(false)
  // }

  fn unallocated(&mut self, tx: &Transaction) -> Result<HashMap<RuneId, Lot>> {
    // map of rune ID to un-allocated balance of that rune
    let mut unallocated: HashMap<RuneId, Lot> = HashMap::new();

    // increment unallocated runes with the runes in tx inputs
    for input in &tx.input {
      if let Some(balances) =
        crate::outpoint_to_rune_balances(|b| b.remove(&OutPoint::store(input.previous_output)))
      {
        for rune in balances.iter() {
          let rune = *rune;
          *unallocated.entry(rune.id).or_default() += rune.balance;
        }
      }
    }
    Ok(unallocated)
  }
}
