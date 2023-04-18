    pub fn print_to(&self, config: &Config) -> CargoResult<()> {
        match self {
            Message::Fixing { file, fixes } => {
                let msg = if *fixes == 1 { "fix" } else { "fixes" };
                let msg = format!("{} ({} {})", file, fixes, msg);
                config.shell().status("Fixing", msg)
            }
            Message::ReplaceFailed { file, message } => {
                let msg = format!("error applying suggestions to `{}`\n", file);
                config.shell().warn(&msg)?;
                write!(
                    config.shell().err(),
                    "The full error message was:\n\n> {}",
                    message,
                )?;
                write!(config.shell().err(), "{}", PLEASE_REPORT_THIS_BUG)?;
                Ok(())
            }
            Message::FixFailed { files, krate } => {
                if let Some(ref krate) = *krate {
                    config.shell().warn(&format!(
                        "failed to automatically apply fixes suggested by rustc \
                         to crate `{}`",
                        krate,
                    ))?;
                } else {
                    config.shell().warn(
                        "failed to automatically apply fixes suggested by rustc"
                    )?;
                }
                if !files.is_empty() {
                    writeln!(
                        config.shell().err(),
                        "\nafter fixes were automatically applied the compiler \
                         reported errors within these files:\n"
                    )?;
                    for file in files {
                        writeln!(config.shell().err(), "  * {}", file)?;
                    }
                    writeln!(config.shell().err())?;
                }
                write!(config.shell().err(), "{}", PLEASE_REPORT_THIS_BUG)?;
                Ok(())
            }
        }

    }
