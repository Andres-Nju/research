	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut map: BTreeMap<String, Value> = BTreeMap::new();
		map.insert("is_first_launch".into(), Value::Bool(self.is_first_launch));
		map.insert("pruning".into(), Value::String(self.pruning.as_str().into()));
		map.insert("tracing".into(), Value::Bool(self.tracing));
		map.insert("fat_db".into(), Value::Bool(self.fat_db));
		let mode_str = match self.mode {
			Mode::Off => "offline",
			Mode::Dark(timeout) => {
				map.insert("mode.timeout".into(), Value::Number(timeout.as_secs().into()));
				"dark"
			},
			Mode::Passive(timeout, alarm) => {
				map.insert("mode.timeout".into(), Value::Number(timeout.as_secs().into()));
				map.insert("mode.alarm".into(), Value::Number(alarm.as_secs().into()));
				"passive"
			},
			Mode::Active => "active",
		};
		map.insert("mode".into(), Value::String(mode_str.into()));

		map.serialize(serializer)
	}
