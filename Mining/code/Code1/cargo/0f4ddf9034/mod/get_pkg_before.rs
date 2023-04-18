    fn get_pkg(&mut self, package: PackageId, path: &File) -> CargoResult<Package> {
        let path = self
            .unpack_package(package, path)
            .chain_err(|| internal(format!("failed to unpack package `{}`", package)))?;
        let mut src = PathSource::new(&path, self.source_id, self.config);
        src.update()?;
        let mut pkg = match src.download(package)? {
            MaybePackage::Ready(pkg) => pkg,
            MaybePackage::Download { .. } => unreachable!(),
        };

        // After we've loaded the package configure it's summary's `checksum`
        // field with the checksum we know for this `PackageId`.
        let req = VersionReq::exact(package.version());
        let summary_with_cksum = self
            .index
            .summaries(package.name(), &req, &mut *self.ops)?
            .map(|s| s.summary.clone())
            .next()
            .expect("summary not found");
        if let Some(cksum) = summary_with_cksum.checksum() {
            pkg.manifest_mut()
                .summary_mut()
                .set_checksum(cksum.to_string());
        }

        Ok(pkg)
    }
