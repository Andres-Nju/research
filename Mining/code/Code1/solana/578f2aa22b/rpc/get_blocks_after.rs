    pub async fn get_blocks(
        &self,
        start_slot: Slot,
        end_slot: Option<Slot>,
        commitment: Option<CommitmentConfig>,
    ) -> Result<Vec<Slot>> {
        let commitment = commitment.unwrap_or_default();
        check_is_at_least_confirmed(commitment)?;

        let highest_confirmed_root = self
            .block_commitment_cache
            .read()
            .unwrap()
            .highest_confirmed_root();

        let end_slot = min(
            end_slot.unwrap_or_else(|| start_slot.saturating_add(MAX_GET_CONFIRMED_BLOCKS_RANGE)),
            if commitment.is_finalized() {
                highest_confirmed_root
            } else {
                self.bank(Some(CommitmentConfig::confirmed())).slot()
            },
        );
        if end_slot < start_slot {
            return Ok(vec![]);
        }
        if end_slot - start_slot > MAX_GET_CONFIRMED_BLOCKS_RANGE {
            return Err(Error::invalid_params(format!(
                "Slot range too large; max {}",
                MAX_GET_CONFIRMED_BLOCKS_RANGE
            )));
        }

        let lowest_blockstore_slot = self.blockstore.lowest_slot();
        if start_slot < lowest_blockstore_slot {
            // If the starting slot is lower than what's available in blockstore assume the entire
            // [start_slot..end_slot] can be fetched from BigTable. This range should not ever run
            // into unfinalized confirmed blocks due to MAX_GET_CONFIRMED_BLOCKS_RANGE
            if let Some(bigtable_ledger_storage) = &self.bigtable_ledger_storage {
                return bigtable_ledger_storage
                    .get_confirmed_blocks(start_slot, (end_slot - start_slot) as usize + 1) // increment limit by 1 to ensure returned range is inclusive of both start_slot and end_slot
                    .await
                    .map(|mut bigtable_blocks| {
                        bigtable_blocks.retain(|&slot| slot <= end_slot);
                        bigtable_blocks
                    })
                    .map_err(|_| {
                        Error::invalid_params(
                            "BigTable query failed (maybe timeout due to too large range?)"
                                .to_string(),
                        )
                    });
            }
        }

        // Finalized blocks
        let mut blocks: Vec<_> = self
            .blockstore
            .rooted_slot_iterator(max(start_slot, lowest_blockstore_slot))
            .map_err(|_| Error::internal_error())?
            .filter(|&slot| slot <= end_slot && slot <= highest_confirmed_root)
            .collect();
        let last_element = blocks
            .last()
            .cloned()
            .unwrap_or_else(|| start_slot.saturating_sub(1));

        // Maybe add confirmed blocks
        if commitment.is_confirmed() && last_element < end_slot {
            let confirmed_bank = self.bank(Some(CommitmentConfig::confirmed()));
            let mut confirmed_blocks = confirmed_bank
                .status_cache_ancestors()
                .into_iter()
                .filter(|&slot| slot <= end_slot && slot > last_element)
                .collect();
            blocks.append(&mut confirmed_blocks);
        }

        Ok(blocks)
    }
