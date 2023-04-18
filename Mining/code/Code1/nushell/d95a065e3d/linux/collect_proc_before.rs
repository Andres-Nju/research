pub fn collect_proc(interval: Duration, with_thread: bool) -> Vec<ProcessInfo> {
    let mut base_procs = Vec::new();
    let mut base_tasks = HashMap::new();
    let mut ret = Vec::new();

    if let Ok(all_proc) = procfs::process::all_processes() {
        for proc in all_proc.flatten() {
            let io = proc.io().ok();
            let time = Instant::now();
            if with_thread {
                if let Ok(iter) = proc.tasks() {
                    collect_task(iter, &mut base_tasks);
                }
                base_procs.push((proc.pid(), proc, io, time));
            }
            // match proc {
            //     Ok(p) => {
            //         let io = p.io().ok();
            //         let time = Instant::now();
            //         if with_thread {
            //             if let Ok(iter) = p.tasks() {
            //                 collect_task(iter, &mut base_tasks);
            //             }
            //         }
            //         base_procs.push((p.pid(), p, io, time));
            //     }
            //     Err(_) => {}
            // }
        }
    }

    thread::sleep(interval);

    for (pid, prev_proc, prev_io, prev_time) in base_procs {
        let curr_proc_pid = pid;
        let prev_proc_pid = prev_proc.pid();
        let curr_proc = match Process::new(curr_proc_pid) {
            Ok(p) => p,
            Err(_) => return Vec::<ProcessInfo>::new(),
        };
        let prev_proc = match Process::new(prev_proc_pid) {
            Ok(p) => p,
            Err(_) => return Vec::<ProcessInfo>::new(),
        };

        let curr_io = curr_proc.io().ok();
        let curr_status = curr_proc.status().ok();
        let curr_time = Instant::now();
        let interval = curr_time - prev_time;
        let ppid = match curr_proc.stat() {
            Ok(p) => p.ppid,
            Err(_) => 0,
        };
        let owner = curr_proc.uid().unwrap_or(0);

        let mut curr_tasks = HashMap::new();
        if with_thread {
            if let Ok(iter) = curr_proc.tasks() {
                collect_task(iter, &mut curr_tasks);
            }
        }

        let curr_proc = ProcessTask::Process(curr_proc);
        let prev_proc = ProcessTask::Process(prev_proc);

        let proc = ProcessInfo {
            pid,
            ppid,
            curr_proc,
            prev_proc,
            curr_io,
            prev_io,
            curr_status,
            interval,
        };

        ret.push(proc);

        for (tid, (pid, curr_stat, curr_status, curr_io)) in curr_tasks {
            if let Some((_, prev_stat, _, prev_io)) = base_tasks.remove(&tid) {
                let proc = ProcessInfo {
                    pid: tid,
                    ppid: pid,
                    curr_proc: ProcessTask::Task {
                        stat: Box::new(curr_stat),
                        owner,
                    },
                    prev_proc: ProcessTask::Task {
                        stat: Box::new(prev_stat),
                        owner,
                    },
                    curr_io,
                    prev_io,
                    curr_status,
                    interval,
                };
                ret.push(proc);
            }
        }
    }

    ret
}
