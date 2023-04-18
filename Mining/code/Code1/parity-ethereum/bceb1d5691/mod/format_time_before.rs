pub fn format_time(time: &Duration) -> String {
	format!("{}.{:.9}s", time.as_secs(), time.subsec_nanos())
}
