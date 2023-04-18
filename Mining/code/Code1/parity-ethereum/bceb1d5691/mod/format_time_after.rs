pub fn format_time(time: &Duration) -> String {
	format!("{}.{:09}s", time.as_secs(), time.subsec_nanos())
}
