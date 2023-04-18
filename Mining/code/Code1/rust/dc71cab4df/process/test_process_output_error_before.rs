    fn test_process_output_error() {
        let Output {status, stdout, stderr}
             = if cfg!(target_os = "windows") {
                 Command::new("cmd").args(&["/C", "mkdir ."]).output().unwrap()
             } else {
                 Command::new("mkdir").arg(".").output().unwrap()
             };

        assert!(status.code() == Some(1));
        assert_eq!(stdout, Vec::new());
        assert!(!stderr.is_empty());
    }
