    pub fn get_new_blockhash(&self, blockhash: &Hash) -> io::Result<(Hash, FeeCalculator)> {
        let mut num_retries = 10;
        while num_retries > 0 {
            if let Ok((new_blockhash, fee_calculator)) = self.get_recent_blockhash() {
                if new_blockhash != *blockhash {
                    return Ok((new_blockhash, fee_calculator));
                }
            }
            debug!("Got same blockhash ({:?}), will retry...", blockhash);

            // Retry ~twice during a slot
            sleep(Duration::from_millis(
                500 * DEFAULT_TICKS_PER_SLOT / DEFAULT_TICKS_PER_SECOND,
            ));
            num_retries -= 1;
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Unable to get new blockhash, too many retries",
        ))
    }
