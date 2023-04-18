fn app_installer_dir(settings: &Settings) -> crate::Result<PathBuf> {
  let arch = match settings.binary_arch() {
     "x86" => "i386",
    "x86_64" => "amd64",
    target => {
      return Err(crate::Error::from(format!(
        "Unsupported architecture: {}",
        target
      )))
    }
  };

  Ok(settings.project_out_directory().to_path_buf().join(format!(
    "{}.{}.msi",
    settings.bundle_name(),
    arch
  )))
}
