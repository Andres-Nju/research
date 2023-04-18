	fn send_message_to_handler() {
		struct MyHandler(atomic::AtomicBool);

		#[derive(Clone)]
		struct MyMessage {
			data: u32
		}

		impl IoHandler<MyMessage> for MyHandler {
			fn message(&self, _io: &IoContext<MyMessage>, message: &MyMessage) {
				assert_eq!(message.data, 5);
				self.0.store(true, atomic::Ordering::SeqCst);
			}
		}

		let handler = Arc::new(MyHandler(atomic::AtomicBool::new(false)));

		let service = IoService::<MyMessage>::start().expect("Error creating network service");
		service.register_handler(handler.clone()).unwrap();

		service.send_message(MyMessage { data: 5 }).unwrap();

		thread::sleep(Duration::from_secs(1));
		assert!(handler.0.load(atomic::Ordering::SeqCst));
	}
