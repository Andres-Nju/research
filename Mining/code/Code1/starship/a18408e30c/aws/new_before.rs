    fn new() -> Self {
        AwsConfig {
            symbol: SegmentConfig::new("☁️ "),
            profile: SegmentConfig::default(),
            style: Color::Yellow.bold(),
            disabled: false,
        }
    }
