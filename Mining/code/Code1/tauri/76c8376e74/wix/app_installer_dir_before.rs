fn app_installer_dir(settings: &Settings) -> crate::Result<PathBuf> {
  let arch = match settings.binary_arch() {
    "x86_64" => "x86",
    "x64" => "x64",
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
