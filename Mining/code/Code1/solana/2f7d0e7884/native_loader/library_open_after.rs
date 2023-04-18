fn library_open(path: &PathBuf) -> std::io::Result<Library> {
    // Linux tls bug can cause crash on dlclose(), workaround by never unloading
    Library::open(Some(path), libc::RTLD_NODELETE | libc::RTLD_NOW)
}
