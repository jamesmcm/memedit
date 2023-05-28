use procfs::ProcResult;

pub struct ProcessRef {
    pub pid: i32,
    pub command: String,
}

/// Get all running PIDs and their command strings
/// Note we don't keep the procfs iterator since this is a lazy iterator
/// and holds a reference to their file descriptors
pub fn get_running_pids() -> ProcResult<Vec<ProcessRef>> {
    procfs::process::all_processes()?
        .filter_map(|opt_p| {
            opt_p
                .map(|p| {
                    p.cmdline().map(|cmd| ProcessRef {
                        pid: p.pid,
                        command: cmd.join(" "),
                    })
                })
                .ok()
        })
        .collect()
}
