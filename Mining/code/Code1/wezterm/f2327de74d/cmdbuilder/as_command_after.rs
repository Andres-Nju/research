    pub(crate) fn as_command(&self) -> anyhow::Result<std::process::Command> {
        let mut cmd = if self.is_default_prog() {
            let mut cmd = std::process::Command::new(&Self::get_shell()?);
            // Run the shell as a login shell.  This is a little shaky; it just
            // happens to be the case that bash, zsh, fish and tcsh use -l
            // to indicate that they are login shells.  Ideally we'd just
            // tell the command builder to prefix argv[0] with a `-`, but
            // Rust doesn't support that.
            cmd.arg("-l");
            let home = Self::get_home_dir()?;
            let dir: &OsStr = self
                .cwd
                .as_ref()
                .map(|dir| dir.as_os_str())
                .filter(|dir| std::path::Path::new(dir).is_dir())
                .unwrap_or(home.as_ref());
            cmd.current_dir(dir);
            cmd
        } else {
            let mut cmd = std::process::Command::new(&self.args[0]);
            cmd.args(&self.args[1..]);
            let home = Self::get_home_dir()?;
            let dir: &OsStr = self
                .cwd
                .as_ref()
                .map(|dir| dir.as_os_str())
                .filter(|dir| std::path::Path::new(dir).is_dir())
                .unwrap_or(home.as_ref());
            cmd.current_dir(dir);
            cmd
        };

        for (key, val) in &self.envs {
            cmd.env(key, val);
        }

        Ok(cmd)
    }
