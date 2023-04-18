pub async fn upgrade(upgrade_flags: UpgradeFlags) -> Result<(), AnyError> {
  let old_exe_path = std::env::current_exe()?;
  let metadata = fs::metadata(&old_exe_path)?;
  let permissions = metadata.permissions();

  if permissions.readonly() {
    bail!(
      "You do not have write permission to {}",
      old_exe_path.display()
    );
  }
  #[cfg(unix)]
  if std::os::unix::fs::MetadataExt::uid(&metadata) == 0
    && !nix::unistd::Uid::effective().is_root()
  {
    bail!(concat!(
      "You don't have write permission to {} because it's owned by root.\n",
      "Consider updating deno through your package manager if its installed from it.\n",
      "Otherwise run `deno upgrade` as root.",
    ), old_exe_path.display());
  }

  let client = build_http_client(upgrade_flags.ca_file)?;

  let install_version = match upgrade_flags.version {
    Some(passed_version) => {
      if upgrade_flags.canary
        && !regex::Regex::new("^[0-9a-f]{40}$")?.is_match(&passed_version)
      {
        bail!("Invalid commit hash passed");
      } else if !upgrade_flags.canary
        && semver::Version::parse(&passed_version).is_err()
      {
        bail!("Invalid semver passed");
      }

      let current_is_passed = if upgrade_flags.canary {
        crate::version::GIT_COMMIT_HASH == passed_version
      } else if !crate::version::is_canary() {
        crate::version::deno() == passed_version
      } else {
        false
      };

      if !upgrade_flags.force
        && upgrade_flags.output.is_none()
        && current_is_passed
      {
        println!("Version {} is already installed", crate::version::deno());
        return Ok(());
      } else {
        passed_version
      }
    }
    None => {
      let latest_version = if upgrade_flags.canary {
        println!("Looking up latest canary version");
        get_latest_canary_version(&client).await?
      } else {
        println!("Looking up latest version");
        get_latest_release_version(&client).await?
      };

      let current_is_most_recent = if upgrade_flags.canary {
        let latest_hash = latest_version.clone();
        crate::version::GIT_COMMIT_HASH == latest_hash
      } else if !crate::version::is_canary() {
        let current = semver::Version::parse(&crate::version::deno()).unwrap();
        let latest = semver::Version::parse(&latest_version).unwrap();
        current >= latest
      } else {
        false
      };

      if !upgrade_flags.force
        && upgrade_flags.output.is_none()
        && current_is_most_recent
      {
        println!(
          "Local deno version {} is the most recent release",
          crate::version::deno()
        );
        return Ok(());
      } else {
        println!("Found latest version {}", latest_version);
        latest_version
      }
    }
  };

  let download_url = if upgrade_flags.canary {
    if env!("TARGET") == "aarch64-apple-darwin" {
      bail!("Canary builds are not available for M1");
    }

    format!(
      "https://dl.deno.land/canary/{}/{}",
      install_version, *ARCHIVE_NAME
    )
  } else {
    format!(
      "{}/download/v{}/{}",
      RELEASE_URL, install_version, *ARCHIVE_NAME
    )
  };

  let archive_data = download_package(client, &download_url).await?;

  println!("Deno is upgrading to version {}", &install_version);

  let new_exe_path = unpack(archive_data, cfg!(windows))?;
  fs::set_permissions(&new_exe_path, permissions)?;
  check_exe(&new_exe_path)?;

  if !upgrade_flags.dry_run {
    match upgrade_flags.output {
      Some(path) => {
        fs::rename(&new_exe_path, &path)
          .or_else(|_| fs::copy(&new_exe_path, &path).map(|_| ()))?;
      }
      None => replace_exe(&new_exe_path, &old_exe_path)?,
    }
  }

  println!("Upgraded successfully");

  Ok(())
}
