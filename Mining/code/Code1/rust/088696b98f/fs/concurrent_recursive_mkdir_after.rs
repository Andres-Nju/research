    fn concurrent_recursive_mkdir() {
        for _ in 0..100 {
            let dir = tmpdir();
            let mut dir = dir.join("a");
            for _ in 0..40 {
                dir = dir.join("a");
            }
            let mut join = vec!();
            for _ in 0..8 {
                let dir = dir.clone();
                join.push(thread::spawn(move || {
                    check!(fs::create_dir_all(&dir));
                }))
            }

            // No `Display` on result of `join()`
            join.drain(..).map(|join| join.join().unwrap()).count();
        }
    }
