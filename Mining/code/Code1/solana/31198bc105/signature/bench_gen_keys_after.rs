    fn bench_gen_keys(b: &mut Bencher) {
        let mut seed = [0u8; 32];
        seed[0..3].copy_from_slice(&[1, 2, 3, 4]);
        let rnd = GenKeys::new(seed);
        b.iter(|| rnd.gen_n_keypairs(1000));
    }
