use chacha::{CHACHA_BLOCK_SIZE, CHACHA_KEY_SIZE};
use hash::Hash;
use ledger::LedgerWindow;
use sigverify::{chacha_cbc_encrypt_many_sample, chacha_end_sha_state, chacha_init_sha_state};
use std::io;
use std::mem::size_of;

const ENTRIES_PER_SLICE: u64 = 16;

// Encrypt a file with multiple starting IV states, determined by ivecs.len()
//
// Then sample each block at the offsets provided by samples argument with sha256
// and return the vec of sha states
pub fn chacha_cbc_encrypt_file_many_keys(
    in_path: &str,
    slice: u64,
    ivecs: &mut [u8],
    samples: &[u64],
) -> io::Result<Vec<Hash>> {
    if ivecs.len() % CHACHA_BLOCK_SIZE != 0 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "bad IV length({}) not divisible by {} ",
                ivecs.len(),
                CHACHA_BLOCK_SIZE,
            ),
        ));
    }

    let mut ledger_window = LedgerWindow::open(in_path)?;
    let mut buffer = [0; 8 * 1024];
    let num_keys = ivecs.len() / CHACHA_BLOCK_SIZE;
    let mut sha_states = vec![0; num_keys * size_of::<Hash>()];
    let mut int_sha_states = vec![0; num_keys * 112];
    let keys: Vec<u8> = vec![0; num_keys * CHACHA_KEY_SIZE]; // keys not used ATM, uniqueness comes from IV
    let mut entry = slice;
    let mut total_entries = 0;
    let mut total_entry_len = 0;
    let mut time: f32 = 0.0;
    unsafe {
        chacha_init_sha_state(int_sha_states.as_mut_ptr(), num_keys as u32);
    }
    loop {
        match ledger_window.get_entries_bytes(entry, ENTRIES_PER_SLICE - total_entries, &mut buffer)
        {
            Ok((num_entries, entry_len)) => {
                info!(
                    "encrypting slice: {} num_entries: {} entry_len: {}",
                    slice, num_entries, entry_len
                );
                let entry_len_usz = entry_len as usize;
                unsafe {
                    chacha_cbc_encrypt_many_sample(
                        buffer[..entry_len_usz].as_ptr(),
                        int_sha_states.as_mut_ptr(),
                        entry_len_usz,
                        keys.as_ptr(),
                        ivecs.as_mut_ptr(),
                        num_keys as u32,
                        samples.as_ptr(),
                        samples.len() as u32,
                        total_entry_len,
                        &mut time,
                    );
                }

                total_entry_len += entry_len;
                total_entries += num_entries;
                entry += num_entries;
                debug!(
                    "total entries: {} entry: {} slice: {} entries_per_slice: {}",
                    total_entries, entry, slice, ENTRIES_PER_SLICE
                );
                if (entry - slice) >= ENTRIES_PER_SLICE {
                    break;
                }
            }
            Err(e) => {
                info!("Error encrypting file: {:?}", e);
                break;
            }
        }
    }
    unsafe {
        chacha_end_sha_state(
            int_sha_states.as_ptr(),
            sha_states.as_mut_ptr(),
            num_keys as u32,
        );
    }
    info!("num_keys: {}", num_keys);
    let mut res = Vec::new();
    for x in 0..num_keys {
        let start = x * size_of::<Hash>();
        let end = start + size_of::<Hash>();
        res.push(Hash::new(&sha_states[start..end]));
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use chacha::chacha_cbc_encrypt_file;
    use chacha_cuda::chacha_cbc_encrypt_file_many_keys;
    use hash::Hash;
    use ledger::LedgerWriter;
    use ledger::{get_tmp_ledger_path, make_tiny_test_entries, LEDGER_DATA_FILE};
    use replicator::sample_file;
    use std::fs::{remove_dir_all, remove_file};
    use std::path::Path;

    #[test]
    fn test_encrypt_file_many_keys_single() {
        use logger;
        logger::setup();

        let entries = make_tiny_test_entries(32);
        let ledger_dir = "test_encrypt_file_many_keys_single";
        let ledger_path = get_tmp_ledger_path(ledger_dir);
        {
            let mut writer = LedgerWriter::open(&ledger_path, true).unwrap();
            writer.write_entries(&entries).unwrap();
        }

        let out_path = Path::new("test_chacha_encrypt_file_many_keys_single_output.txt.enc");

        let samples = [0];
        let mut ivecs = hex!(
            "abcd1234abcd1234abcd1234abcd1234 abcd1234abcd1234abcd1234abcd1234
                              abcd1234abcd1234abcd1234abcd1234 abcd1234abcd1234abcd1234abcd1234"
        );

        let mut cpu_iv = ivecs.clone();
        assert!(
            chacha_cbc_encrypt_file(
                &Path::new(&ledger_path).join(LEDGER_DATA_FILE),
                out_path,
                &mut cpu_iv,
            ).is_ok()
        );

        let ref_hash = sample_file(&out_path, &samples).unwrap();

        let hashes =
            chacha_cbc_encrypt_file_many_keys(&ledger_path, 0, &mut ivecs, &samples).unwrap();

        assert_eq!(hashes[0], ref_hash);

        let _ignored = remove_dir_all(&ledger_path);
        let _ignored = remove_file(out_path);
    }

    #[test]
    fn test_encrypt_file_many_keys_multiple_keys() {
        use logger;
        logger::setup();

        let entries = make_tiny_test_entries(32);
        let ledger_dir = "test_encrypt_file_many_keys_multiple";
        let ledger_path = get_tmp_ledger_path(ledger_dir);
        {
            let mut writer = LedgerWriter::open(&ledger_path, true).unwrap();
            writer.write_entries(&entries).unwrap();
        }

        let out_path = Path::new("test_chacha_encrypt_file_many_keys_multiple_output.txt.enc");

        let samples = [0, 1, 3, 4, 5, 150];
        let mut ivecs = Vec::new();
        let mut ref_hashes: Vec<Hash> = vec![];
        for i in 0..2 {
            let mut ivec = hex!(
                "abc123abc123abc123abc123abc123abc123abababababababababababababab
                                 abc123abc123abc123abc123abc123abc123abababababababababababababab"
            );
            ivec[0] = i;
            ivecs.extend(ivec.clone().iter());
            assert!(
                chacha_cbc_encrypt_file(
                    &Path::new(&ledger_path).join(LEDGER_DATA_FILE),
                    out_path,
                    &mut ivec,
                ).is_ok()
            );

            ref_hashes.push(sample_file(&out_path, &samples).unwrap());
            info!(
                "ivec: {:?} hash: {:?} ivecs: {:?}",
                ivec.to_vec(),
                ref_hashes.last(),
                ivecs
            );
        }

        let hashes =
            chacha_cbc_encrypt_file_many_keys(&ledger_path, 0, &mut ivecs, &samples).unwrap();

        assert_eq!(hashes, ref_hashes);

        let _ignored = remove_dir_all(&ledger_path);
        let _ignored = remove_file(out_path);
    }

    #[test]
    fn test_encrypt_file_many_keys_bad_key_length() {
        let mut keys = hex!("abc123");
        let ledger_dir = "test_encrypt_file_many_keys_bad_key_length";
        let ledger_path = get_tmp_ledger_path(ledger_dir);
        let samples = [0];
        assert!(chacha_cbc_encrypt_file_many_keys(&ledger_path, 0, &mut keys, &samples,).is_err());
    }
}
