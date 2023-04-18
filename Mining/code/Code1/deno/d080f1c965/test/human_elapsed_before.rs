fn human_elapsed(elapsed: u128) -> String {
  if elapsed < 1_000 {
    return format!("({}ms)", elapsed);
  }
  if elapsed < 1_000 * 60 {
    return format!("({}s)", elapsed / 1000);
  }

  let seconds = elapsed / 1_000;
  let minutes = seconds / 60;
  let seconds_reminder = seconds % 60;
  format!("({}m{}s)", minutes, seconds_reminder)
}
