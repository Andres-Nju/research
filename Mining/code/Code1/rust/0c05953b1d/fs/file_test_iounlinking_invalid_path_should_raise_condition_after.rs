    fn file_test_iounlinking_invalid_path_should_raise_condition() {
        let tmpdir = tmpdir();
        let filename = &tmpdir.join("file_another_file_that_does_not_exist.txt");

        let result = fs::remove_file(filename);

        if cfg!(unix) {
            error!(result, "No such file or directory");
        }
        if cfg!(windows) {
            error!(result, "The system cannot find the file specified");
        }
    }
