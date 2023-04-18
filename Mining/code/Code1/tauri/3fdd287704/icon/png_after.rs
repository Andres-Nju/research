fn png(source: &DynamicImage, out_dir: &Path) -> Result<()> {
  for size in [32, 128, 256, 512] {
    let file_name = match size {
      256 => "128x128@2x.png".to_string(),
      512 => "icon.png".to_string(),
      _ => format!("{}x{}.png", size, size),
    };

    log::info!(action = "PNG"; "Creating {}", file_name);
    resize_and_save_png(source, size, &out_dir.join(&file_name))?;
  }

  Ok(())
}
