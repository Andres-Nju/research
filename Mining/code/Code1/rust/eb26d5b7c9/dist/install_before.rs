fn install(src: &Path, dstdir: &Path, perms: u32) {
    let dst = dstdir.join(src.file_name().unwrap());
    t!(fs::create_dir_all(dstdir));
    {
        let mut s = t!(fs::File::open(&src));
        let mut d = t!(fs::File::create(&dst));
        io::copy(&mut s, &mut d).expect("failed to copy");
    }
    chmod(&dst, perms);
}
