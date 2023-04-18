    pub fn discover(path: &Path, cwd: &Path) -> CargoResult<HgRepo> {
        process("hg")
            .cwd(cwd)
            .arg("--cwd")
            .arg(path)
            .arg("root")
            .exec_with_output()?;
        Ok(HgRepo)
    }
