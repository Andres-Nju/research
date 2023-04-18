    fn build(&mut self) {
        self.rust_version = self.version("rust", "x86_64-unknown-linux-gnu");
        self.cargo_version = self.version("cargo", "x86_64-unknown-linux-gnu");

        self.digest_and_sign();
        let Manifest { manifest_version, date, pkg } = self.build_manifest();

        // Unfortunately we can't use derive(RustcEncodable) here because the
        // version field is called `manifest-version`, not `manifest_version`.
        // In lieu of that just create the table directly here with a `BTreeMap`
        // and wrap it up in a `Value::Table`.
        let mut manifest = BTreeMap::new();
        manifest.insert("manifest-version".to_string(),
                        toml::Value::String(manifest_version));
        manifest.insert("date".to_string(), toml::Value::String(date));
        manifest.insert("pkg".to_string(), toml::encode(&pkg));
        let manifest = toml::Value::Table(manifest).to_string();

        let filename = format!("channel-rust-{}.toml", self.channel);
        self.write_manifest(&manifest, &filename);

        if self.channel != "beta" && self.channel != "nightly" {
            self.write_manifest(&manifest, "channel-rust-stable.toml");
        }
    }
