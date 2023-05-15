    fn install(&self, src: &Path, dstdir: &Path, perms: u32) {
        if self.config.dry_run { return; }
        let dst = dstdir.join(src.file_name().unwrap());
        t!(fs::create_dir_all(dstdir));
        drop(fs::remove_file(&dst));
        {
            if !src.exists() {
                panic!("Error: File \"{}\" not found!", src.display());
            }
            let mut s = t!(fs::File::open(&src));
            let mut d = t!(fs::File::create(&dst));
            io::copy(&mut s, &mut d).expect("failed to copy");
        }
        chmod(&dst, perms);
    }