    pub fn sleep(dur: Duration) {
        let nanos = dur.as_nanos();
        assert!(nanos <= u64::max_value() as u128);

        const USERDATA: wasi::Userdata = 0x0123_45678;

        let clock = wasi::raw::__wasi_subscription_u_clock_t {
            identifier: 0,
            clock_id: wasi::CLOCK_MONOTONIC,
            timeout: nanos as u64,
            precision: 0,
            flags: 0,
        };

        let in_ = [wasi::Subscription {
            userdata: USERDATA,
            type_: wasi::EVENTTYPE_CLOCK,
            u: wasi::raw::__wasi_subscription_u { clock: clock },
        }];
        let (res, event) = unsafe {
            let mut out: [wasi::Event; 1] = mem::zeroed();
            let res = wasi::poll_oneoff(&in_, &mut out);
            (res, out[0])
        };
        match (res, event) {
            (Ok(1), wasi::Event {
                userdata: USERDATA,
                error: 0,
                type_: wasi::EVENTTYPE_CLOCK,
                ..
            }) => {}
            _ => panic!("thread::sleep(): unexpected result of poll_oneoff"),
        }
    }
