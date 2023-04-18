pub fn set_panic_hook(program: &'static str) {
    use std::panic;
    use std::sync::{Once, ONCE_INIT};
    static SET_HOOK: Once = ONCE_INIT;
    SET_HOOK.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |ono| {
            default_hook(ono);
            submit(
                influxdb::Point::new("panic")
                    .add_tag("program", influxdb::Value::String(program.to_string()))
                    .add_tag(
                        "thread",
                        influxdb::Value::String(
                            thread::current().name().unwrap_or("?").to_string(),
                        ),
                    )
                    // The 'one' field exists to give Kapacitor Alerts a numerical value
                    // to filter on
                    .add_field("one", influxdb::Value::Integer(1))
                    .add_field(
                        "message",
                        influxdb::Value::String(
                            // TODO: use ono.message() when it becomes stable
                            ono.to_string(),
                        ),
                    )
                    .add_field(
                        "location",
                        influxdb::Value::String(match ono.location() {
                            Some(location) => location.to_string(),
                            None => "?".to_string(),
                        }),
                    )
                    .add_field("host_id", influxdb::Value::String(HOST_INFO.to_string()))
                    .to_owned(),
            );
            // Flush metrics immediately in case the process exits immediately
            // upon return
            flush();
        }));
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockMetricsWriter {
        points_written: AtomicUsize,
    }
    impl MockMetricsWriter {
        fn new() -> Self {
            MockMetricsWriter {
                points_written: AtomicUsize::new(0),
            }
        }

        fn points_written(&self) -> usize {
            return self.points_written.load(Ordering::SeqCst);
        }
    }

    impl MetricsWriter for MockMetricsWriter {
        fn write(&self, points: Vec<influxdb::Point>) {
            assert!(!points.is_empty());

            self.points_written
                .fetch_add(points.len(), Ordering::SeqCst);

            info!(
                "Writing {} points ({} total)",
                points.len(),
                self.points_written.load(Ordering::SeqCst)
            );
        }
    }

    #[test]
    fn test_submit() {
        let writer = Arc::new(MockMetricsWriter::new());
        let agent = MetricsAgent::new(writer.clone(), Duration::from_secs(10));

        for i in 0..42 {
            agent.submit(influxdb::Point::new(&format!("measurement {}", i)));
        }

        agent.flush();
        assert_eq!(writer.points_written(), 42);
    }

    #[test]
    fn test_submit_with_delay() {
        let writer = Arc::new(MockMetricsWriter::new());
        let agent = MetricsAgent::new(writer.clone(), Duration::from_millis(100));

        agent.submit(influxdb::Point::new("point 1"));
        thread::sleep(Duration::from_secs(2));
        assert_eq!(writer.points_written(), 1);
    }

    #[test]
    fn test_multithread_submit() {
        let writer = Arc::new(MockMetricsWriter::new());
        let agent = Arc::new(Mutex::new(MetricsAgent::new(
            writer.clone(),
            Duration::from_secs(10),
        )));

        //
        // Submit measurements from different threads
        //
        let mut threads = Vec::new();
        for i in 0..42 {
            let point = influxdb::Point::new(&format!("measurement {}", i));
            let agent = Arc::clone(&agent);
            threads.push(thread::spawn(move || {
                agent.lock().unwrap().submit(point);
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }

        agent.lock().unwrap().flush();
        assert_eq!(writer.points_written(), 42);
    }

    #[test]
    fn test_flush_before_drop() {
        let writer = Arc::new(MockMetricsWriter::new());
        {
            let agent = MetricsAgent::new(writer.clone(), Duration::from_secs(9999999));
            agent.submit(influxdb::Point::new("point 1"));
        }

        assert_eq!(writer.points_written(), 1);
    }

    #[test]
    fn test_live_submit() {
        let agent = MetricsAgent::default();

        let point = influxdb::Point::new("live_submit_test")
            .add_tag("test", influxdb::Value::Boolean(true))
            .add_field(
                "random_bool",
                influxdb::Value::Boolean(rand::random::<u8>() < 128),
            )
            .add_field(
                "random_int",
                influxdb::Value::Integer(rand::random::<u8>() as i64),
            )
            .to_owned();
        agent.submit(point);
    }

}
