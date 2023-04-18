fn add_to_path(new_path: &str) -> bool {
    let shell_export_string = format!("\nexport PATH=\"{}:$PATH\"", new_path);
    let mut modified_rcfiles = false;

    // Look for sh, bash, and zsh rc files
    let mut rcfiles = vec![dirs_next::home_dir().map(|p| p.join(".profile"))];
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            let zdotdir = std::env::var("ZDOTDIR")
                .ok()
                .map(PathBuf::from)
                .or_else(dirs_next::home_dir);
            let zprofile = zdotdir.map(|p| p.join(".zprofile"));
            rcfiles.push(zprofile);
        }
    }

    if let Some(bash_profile) = dirs_next::home_dir().map(|p| p.join(".bash_profile")) {
        // Only update .bash_profile if it exists because creating .bash_profile
        // will cause .profile to not be read
        if bash_profile.exists() {
            rcfiles.push(Some(bash_profile));
        }
    }
    let rcfiles = rcfiles.into_iter().filter_map(|f| f.filter(|f| f.exists()));

    // For each rc file, append a PATH entry if not already present
    for rcfile in rcfiles {
        if !rcfile.exists() {
            continue;
        }

        fn read_file(path: &Path) -> io::Result<String> {
            let mut file = fs::OpenOptions::new().read(true).open(path)?;
            let mut contents = String::new();
            io::Read::read_to_string(&mut file, &mut contents)?;
            Ok(contents)
        }

        match read_file(&rcfile) {
            Err(err) => {
                println!("Unable to read {:?}: {}", rcfile, err);
            }
            Ok(contents) => {
                if !contents.contains(&shell_export_string) {
                    println!(
                        "Adding {} to {}",
                        style(&shell_export_string).italic(),
                        style(rcfile.to_str().unwrap()).bold()
                    );

                    fn append_file(dest: &Path, line: &str) -> io::Result<()> {
                        use std::io::Write;
                        let mut dest_file = fs::OpenOptions::new()
                            .write(true)
                            .append(true)
                            .create(true)
                            .open(dest)?;

                        writeln!(&mut dest_file, "{}", line)?;

                        dest_file.sync_data()?;

                        Ok(())
                    }
                    append_file(&rcfile, &shell_export_string).unwrap_or_else(|err| {
                        format!("Unable to append to {:?}: {}", rcfile, err);
                    });
                    modified_rcfiles = true;
                }
            }
        }
    }

    if modified_rcfiles {
        println!(
            "\n{}\n  {}\n",
            style("Close and reopen your terminal to apply the PATH changes or run the following in your existing shell:").bold().blue(),
            shell_export_string
       );
    }

    modified_rcfiles
}
