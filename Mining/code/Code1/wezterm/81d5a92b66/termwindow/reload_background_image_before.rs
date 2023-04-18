fn reload_background_image(
    config: &ConfigHandle,
    image: &Option<Arc<ImageData>>,
) -> Option<Arc<ImageData>> {
    match &config.window_background_image {
        Some(p) => match std::fs::read(p) {
            Ok(data) => {
                if let Some(existing) = image {
                    if existing.data() == data {
                        return Some(Arc::clone(existing));
                    }
                }
                Some(Arc::new(ImageData::with_raw_data(data)))
            }
            Err(err) => {
                log::error!(
                    "Failed to load window_background_image {}: {}",
                    p.display(),
                    err
                );
                None
            }
        },
        None => None,
    }
}
