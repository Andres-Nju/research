    fn compare(&self, old: &Fingerprint) -> CargoResult<()> {
        if self.rustc != old.rustc {
            bail!("rust compiler has changed")
        }
        if self.features != old.features {
            bail!(
                "features have changed: {} != {}",
                self.features,
                old.features
            )
        }
        if self.target != old.target {
            bail!("target configuration has changed")
        }
        if self.path != old.path {
            bail!("path to the compiler has changed")
        }
        if self.profile != old.profile {
            bail!("profile configuration has changed")
        }
        if self.rustflags != old.rustflags {
            bail!("RUSTFLAGS has changed")
        }
        if self.metadata != old.metadata {
            bail!("metadata changed")
        }
        let my_local = self.local.lock().unwrap();
        let old_local = old.local.lock().unwrap();
        if my_local.len() != old_local.len() {
            bail!("local lens changed");
        }
        for (new, old) in my_local.iter().zip(old_local.iter()) {
            match (new, old) {
                (LocalFingerprint::Precalculated(a), LocalFingerprint::Precalculated(b)) => {
                    if a != b {
                        bail!("precalculated components have changed: {} != {}", a, b)
                    }
                }
                (
                    LocalFingerprint::CheckDepInfo { dep_info: adep },
                    LocalFingerprint::CheckDepInfo { dep_info: bdep },
                ) => {
                    if adep != bdep {
                        bail!("dep info output changed: {:?} != {:?}", adep, bdep)
                    }
                }
                (
                    LocalFingerprint::RerunIfChanged {
                        output: aout,
                        paths: apaths,
                    },
                    LocalFingerprint::RerunIfChanged {
                        output: bout,
                        paths: bpaths,
                    },
                ) => {
                    if aout != bout {
                        bail!("rerun-if-changed output changed: {:?} != {:?}", aout, bout)
                    }
                    if apaths != bpaths {
                        bail!(
                            "rerun-if-changed output changed: {:?} != {:?}",
                            apaths,
                            bpaths,
                        )
                    }
                }
                (
                    LocalFingerprint::RerunIfEnvChanged {
                        var: akey,
                        val: avalue,
                    },
                    LocalFingerprint::RerunIfEnvChanged {
                        var: bkey,
                        val: bvalue,
                    },
                ) => {
                    if *akey != *bkey {
                        bail!("env vars changed: {} != {}", akey, bkey);
                    }
                    if *avalue != *bvalue {
                        bail!(
                            "env var `{}` changed: previously {:?} now {:?}",
                            akey,
                            bvalue,
                            avalue
                        )
                    }
                }
                (a, b) => bail!(
                    "local fingerprint type has changed ({} => {})",
                    b.kind(),
                    a.kind()
                ),
            }
        }

        if self.deps.len() != old.deps.len() {
            bail!("number of dependencies has changed")
        }
        for (a, b) in self.deps.iter().zip(old.deps.iter()) {
            if a.name != b.name {
                let e = format_err!("`{}` != `{}`", a.name, b.name)
                    .context("unit dependency name changed");
                return Err(e.into());
            }

            if a.fingerprint.hash() != b.fingerprint.hash() {
                let e = format_err!(
                    "new ({}/{:x}) != old ({}/{:x})",
                    a.name,
                    a.fingerprint.hash(),
                    b.name,
                    b.fingerprint.hash()
                )
                .context("unit dependency information changed");
                return Err(e.into());
            }
        }

        if !self.fs_status.up_to_date() {
            bail!("current filesystem status shows we're outdated");
        }

        // This typically means some filesystem modifications happened or
        // something transitive was odd. In general we should strive to provide
        // a better error message than this, so if you see this message a lot it
        // likely means this method needs to be updated!
        bail!("two fingerprint comparison turned up nothing obvious");
    }
