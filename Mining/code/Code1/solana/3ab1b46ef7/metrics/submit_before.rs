    pub fn submit(&self, mut point: influxdb::Point, level: log::Level) {
        point.add_tag("host_id", influxdb::Value::String(HOST_ID.to_string()));
        if point.timestamp.is_none() {
            point.timestamp = Some(timing::timestamp() as i64);
        }
        debug!("Submitting point: {:?}", point);
        self.sender
            .send(MetricsCommand::Submit(point, level))
            .unwrap();
    }
