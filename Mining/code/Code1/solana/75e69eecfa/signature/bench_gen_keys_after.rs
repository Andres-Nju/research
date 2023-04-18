    fn bench_gen_keys(b: &mut Bencher) {
        let seed: &[_] = &[1, 2, 3, 4];
        let rnd = GenKeys::new(seed);
        b.iter(|| rnd.gen_n_keypairs(1000));
    }
