fn enabled() -> bool {
    use winapi::um::{handleapi, jobapi, jobapi2, processthreadsapi};

    unsafe {
        // If we're not currently in a job, then we can definitely run these
        // tests.
        let me = processthreadsapi::GetCurrentProcess();
        let mut ret = 0;
        let r = jobapi::IsProcessInJob(me, 0 as *mut _, &mut ret);
        assert_ne!(r, 0);
        if ret == ::winapi::shared::minwindef::FALSE {
            return true
        }

        // If we are in a job, then we can run these tests if we can be added to
        // a nested job (as we're going to create a nested job no matter what as
        // part of these tests.
        //
        // If we can't be added to a nested job, then these tests will
        // definitely fail, and there's not much we can do about that.
        let job = jobapi2::CreateJobObjectW(0 as *mut _, 0 as *const _);
        assert!(!job.is_null());
        let r = jobapi2::AssignProcessToJobObject(job, me);
        handleapi::CloseHandle(job);
        r != 0
    }
}
