fn tauri_config_to_bundle_settings(
  manifest: &Manifest,
  config: crate::helpers::config::BundleConfig,
  system_tray_config: Option<crate::helpers::config::SystemTrayConfig>,
  updater_config: crate::helpers::config::UpdaterConfig,
) -> crate::Result<BundleSettings> {
  #[cfg(windows)]
  let windows_icon_path = PathBuf::from(
    config
      .icon
      .as_ref()
      .and_then(|icons| icons.iter().find(|i| i.ends_with(".ico")).cloned())
      .expect("the bundle config must have a `.ico` icon"),
  );
  #[cfg(not(windows))]
  let windows_icon_path = PathBuf::from("");

  #[allow(unused_mut)]
  let mut resources = config.resources.unwrap_or_default();
  #[allow(unused_mut)]
  let mut depends = config.deb.depends.unwrap_or_default();

  #[cfg(target_os = "linux")]
  {
    if let Some(system_tray_config) = &system_tray_config {
      let mut icon_path = system_tray_config.icon_path.clone();
      icon_path.set_extension("png");
      depends.push("libappindicator3-1".to_string());
    }

    depends.push("libwebkit2gtk-4.0".to_string());
    depends.push("libgtk-3-0".to_string());
    if manifest.features.contains(&"menu".into()) || system_tray_config.is_some() {
      depends.push("libgtksourceview-3.0-1".to_string());
    }
  }

  Ok(BundleSettings {
    identifier: config.identifier,
    icon: config.icon,
    resources: if resources.is_empty() {
      None
    } else {
      Some(resources)
    },
    copyright: config.copyright,
    category: match config.category {
      Some(category) => Some(AppCategory::from_str(&category).map_err(|e| match e {
        Some(e) => anyhow::anyhow!("invalid category, did you mean `{}`?", e),
        None => anyhow::anyhow!("invalid category"),
      })?),
      None => None,
    },
    short_description: config.short_description,
    long_description: config.long_description,
    external_bin: config.external_bin,
    deb: DebianSettings {
      depends: if depends.is_empty() {
        None
      } else {
        Some(depends)
      },
      use_bootstrapper: Some(config.deb.use_bootstrapper),
      files: config.deb.files,
    },
    macos: MacOsSettings {
      frameworks: config.macos.frameworks,
      minimum_system_version: config.macos.minimum_system_version,
      license: config.macos.license,
      use_bootstrapper: Some(config.macos.use_bootstrapper),
      exception_domain: config.macos.exception_domain,
      signing_identity: config.macos.signing_identity,
      entitlements: config.macos.entitlements,
    },
    windows: WindowsSettings {
      timestamp_url: config.windows.timestamp_url,
      digest_algorithm: config.windows.digest_algorithm,
      certificate_thumbprint: config.windows.certificate_thumbprint,
      wix: config.windows.wix.map(|w| w.into()),
      icon_path: windows_icon_path,
    },
    updater: Some(UpdaterSettings {
      active: updater_config.active,
      // we set it to true by default we shouldn't have to use
      // unwrap_or as we have a default value but used to prevent any failing
      dialog: updater_config.dialog.unwrap_or(true),
      pubkey: updater_config.pubkey,
      endpoints: updater_config.endpoints,
    }),
    ..Default::default()
  })
}
