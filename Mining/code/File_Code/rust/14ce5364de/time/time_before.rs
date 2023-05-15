use cmp::Ordering;
use fmt;
use mem;
use sys::c;
use time::Duration;
use convert::TryInto;
use core::hash::{Hash, Hasher};

const NANOS_PER_SEC: u64 = 1_000_000_000;
const INTERVALS_PER_SEC: u64 = NANOS_PER_SEC / 100;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Instant {
    t: Duration,
}

#[derive(Copy, Clone)]
pub struct SystemTime {
    t: c::FILETIME,
}

const INTERVALS_TO_UNIX_EPOCH: u64 = 11_644_473_600 * INTERVALS_PER_SEC;

pub const UNIX_EPOCH: SystemTime = SystemTime {
    t: c::FILETIME {
        dwLowDateTime: INTERVALS_TO_UNIX_EPOCH as u32,
        dwHighDateTime: (INTERVALS_TO_UNIX_EPOCH >> 32) as u32,
    },
};

impl Instant {
    pub fn now() -> Instant {
        // High precision timing on windows operates in "Performance Counter"
        // units, as returned by the WINAPI QueryPerformanceCounter function.
        // These relate to seconds by a factor of QueryPerformanceFrequency.
        // In order to keep unit conversions out of normal interval math, we
        // measure in QPC units and immediately convert to nanoseconds.
        perf_counter::PerformanceCounterInstant::now().into()
    }

    pub fn actually_monotonic() -> bool {
        false
    }

    pub const fn zero() -> Instant {
        Instant { t: Duration::from_secs(0) }
    }

    pub fn sub_instant(&self, other: &Instant) -> Duration {
        // On windows there's a threshold below which we consider two timestamps
        // equivalent due to measurement error. For more details + doc link,
        // check the docs on epsilon.
        let epsilon =
            perf_counter::PerformanceCounterInstant::epsilon();
        if other.t > self.t && other.t - self.t <= epsilon {
            return Duration::new(0, 0)
        }
        self.t.checked_sub(other.t)
              .expect("specified instant was later than self")
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant {
            t: self.t.checked_add(*other)?
        })
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<Instant> {
        Some(Instant {
            t: self.t.checked_sub(*other)?
        })
    }
}

impl SystemTime {
    pub fn now() -> SystemTime {
        unsafe {
            let mut t: SystemTime = mem::zeroed();
            c::GetSystemTimeAsFileTime(&mut t.t);
            return t
        }
    }

    fn from_intervals(intervals: i64) -> SystemTime {
        SystemTime {
            t: c::FILETIME {
                dwLowDateTime: intervals as c::DWORD,
                dwHighDateTime: (intervals >> 32) as c::DWORD,
            }
        }
    }

    fn intervals(&self) -> i64 {
        (self.t.dwLowDateTime as i64) | ((self.t.dwHighDateTime as i64) << 32)
    }

    pub fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        let me = self.intervals();
        let other = other.intervals();
        if me >= other {
            Ok(intervals2dur((me - other) as u64))
        } else {
            Err(intervals2dur((other - me) as u64))
        }
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<SystemTime> {
        let intervals = self.intervals().checked_add(checked_dur2intervals(other)?)?;
        Some(SystemTime::from_intervals(intervals))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<SystemTime> {
        let intervals = self.intervals().checked_sub(checked_dur2intervals(other)?)?;
        Some(SystemTime::from_intervals(intervals))
    }
}

impl PartialEq for SystemTime {
    fn eq(&self, other: &SystemTime) -> bool {
        self.intervals() == other.intervals()
    }
}

impl Eq for SystemTime {}

impl PartialOrd for SystemTime {
    fn partial_cmp(&self, other: &SystemTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SystemTime {
    fn cmp(&self, other: &SystemTime) -> Ordering {
        self.intervals().cmp(&other.intervals())
    }
}

impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SystemTime")
         .field("intervals", &self.intervals())
         .finish()
    }
}

impl From<c::FILETIME> for SystemTime {
    fn from(t: c::FILETIME) -> SystemTime {
        SystemTime { t }
    }
}

impl Hash for SystemTime {
    fn hash<H : Hasher>(&self, state: &mut H) {
        self.intervals().hash(state)
    }
}

fn checked_dur2intervals(dur: &Duration) -> Option<i64> {
    dur.as_secs()
        .checked_mul(INTERVALS_PER_SEC)?
        .checked_add(dur.subsec_nanos() as u64 / 100)?
        .try_into()
        .ok()
}

fn intervals2dur(intervals: u64) -> Duration {
    Duration::new(intervals / INTERVALS_PER_SEC,
                  ((intervals % INTERVALS_PER_SEC) * 100) as u32)
}

mod perf_counter {
    use super::{NANOS_PER_SEC};
    use sync::Once;
    use sys_common::mul_div_u64;
    use sys::c;
    use sys::cvt;
    use time::Duration;

    pub struct PerformanceCounterInstant {
        ts: c::LARGE_INTEGER
    }
    impl PerformanceCounterInstant {
        pub fn now() -> Self {
            Self {
                ts: query()
            }
        }

        // Per microsoft docs, the margin of error for cross-thread time comparisons
        // using QueryPerformanceCounter is 1 "tick" -- defined as 1/frequency().
        // Reference: https://docs.microsoft.com/en-us/windows/desktop/SysInfo
        //                   /acquiring-high-resolution-time-stamps
        pub fn epsilon() -> Duration {
            let epsilon = NANOS_PER_SEC / (frequency() as u64);
            Duration::from_nanos(epsilon)
        }
    }
    impl From<PerformanceCounterInstant> for super::Instant {
        fn from(other: PerformanceCounterInstant) -> Self {
            let freq = frequency() as u64;
            let instant_nsec = mul_div_u64(other.ts as u64, NANOS_PER_SEC, freq);
            Self {
                t: Duration::from_nanos(instant_nsec)
            }
        }
    }

    fn frequency() -> c::LARGE_INTEGER {
        static mut FREQUENCY: c::LARGE_INTEGER = 0;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                cvt(c::QueryPerformanceFrequency(&mut FREQUENCY)).unwrap();
            });
            FREQUENCY
        }
    }

    fn query() -> c::LARGE_INTEGER {
        let mut qpc_value: c::LARGE_INTEGER = 0;
        cvt(unsafe {
            c::QueryPerformanceCounter(&mut qpc_value)
        }).unwrap();
        qpc_value
    }
}