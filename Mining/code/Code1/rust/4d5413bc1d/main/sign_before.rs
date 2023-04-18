    fn sign(&self, path: &Path) {
        if !self.should_sign {
            return;
        }

        let filename = path.file_name().unwrap().to_str().unwrap();
        let asc = self.output.join(format!("{}.asc", filename));
        println!("signing: {:?}", path);
        let mut cmd = Command::new("gpg");
        cmd.arg("--no-tty")
            .arg("--yes")
            .arg("--passphrase-fd").arg("0")
            .arg("--personal-digest-preferences").arg("SHA512")
            .arg("--armor")
            .arg("--output").arg(&asc)
            .arg("--detach-sign").arg(path)
            .stdin(Stdio::piped());
        let mut child = t!(cmd.spawn());
        t!(child.stdin.take().unwrap().write_all(self.gpg_passphrase.as_bytes()));
        assert!(t!(child.wait()).success());
    }
