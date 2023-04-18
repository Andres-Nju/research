    pub fn discover(path: &Path, cwd: &Path) -> CargoResult<HgRepo> {
        process("hg")
            .cwd(cwd)
            .arg("root")
            .cwd(path)
            .exec_with_output()?;
        Ok(HgRepo)
    }
