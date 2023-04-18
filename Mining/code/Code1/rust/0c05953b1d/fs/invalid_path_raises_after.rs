    fn invalid_path_raises() {
        let tmpdir = tmpdir();
        let filename = &tmpdir.join("file_that_does_not_exist.txt");
        let result = File::open(filename);

        if cfg!(unix) {
            error!(result, "No such file or directory");
        }
        if cfg!(windows) {
            error!(result, "The system cannot find the file specified");
        }
    }
