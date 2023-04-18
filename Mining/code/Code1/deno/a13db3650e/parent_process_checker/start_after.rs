pub fn start(parent_process_id: u32) {
  tokio::task::spawn(async move {
    loop {
      sleep(Duration::from_secs(30)).await;

      if !is_process_active(parent_process_id) {
        std::process::exit(1);
      }
    }
  });
}

