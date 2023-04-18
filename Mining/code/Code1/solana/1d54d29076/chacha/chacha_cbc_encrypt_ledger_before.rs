pub fn chacha_cbc_encrypt_ledger(
    blocktree: &Arc<Blocktree>,
    slice: u64,
    out_path: &Path,
    ivec: &mut [u8; CHACHA_BLOCK_SIZE],
) -> io::Result<usize> {
    let mut out_file =
        BufWriter::new(File::create(out_path).expect("Can't open ledger encrypted data file"));
    const BUFFER_SIZE: usize = 8 * 1024;
    let mut buffer = [0; BUFFER_SIZE];
    let mut encrypted_buffer = [0; BUFFER_SIZE];
    let key = [0; CHACHA_KEY_SIZE];
    let mut total_entries = 0;
    let mut total_size = 0;
    let mut entry = slice;

    loop {
        match blocktree.read_blobs_bytes(entry, SLOTS_PER_SEGMENT - total_entries, &mut buffer, 0) {
            Ok((num_entries, entry_len)) => {
                debug!(
                    "chacha: encrypting slice: {} num_entries: {} entry_len: {}",
                    slice, num_entries, entry_len
                );
                debug!("read {} bytes", entry_len);
                let mut size = entry_len as usize;
                if size == 0 {
                    break;
                }

                if size < BUFFER_SIZE {
                    // We are on the last block, round to the nearest key_size
                    // boundary
                    size = (size + CHACHA_KEY_SIZE - 1) & !(CHACHA_KEY_SIZE - 1);
                }
                total_size += size;

                chacha_cbc_encrypt(&buffer[..size], &mut encrypted_buffer[..size], &key, ivec);
                if let Err(res) = out_file.write(&encrypted_buffer[..size]) {
                    warn!("Error writing file! {:?}", res);
                    return Err(res);
                }

                total_entries += num_entries;
                entry += num_entries;
            }
            Err(e) => {
                info!("Error encrypting file: {:?}", e);
                break;
            }
        }
    }
    Ok(total_size)
}
