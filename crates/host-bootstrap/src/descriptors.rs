//! Process capabilities use the runner's existing Unix descriptor carrier at the exec boundary.
#[cfg(not(verus_keep_ghost))]
use std::{ffi::OsStr, path::Path, process::Command};
#[cfg(not(verus_keep_ghost))]
pub const ENV: &str = "CHRONO_PROCESS_FDS";

#[cfg(not(verus_keep_ghost))]
pub fn available() -> Result<bool, String> {
    let out = Command::new("chrono-harness")
        .arg("inherited-descriptors-v1")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr)
            .trim_end_matches('\n')
            .to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

/// The helper replaces itself with the actual target, preserving the operation PID and native streams.
#[cfg(not(verus_keep_ghost))]
pub fn command(
    root: &Path,
    program: &Path,
    args: &[&str],
    environment: &crate::bindings::Environment,
) -> Result<Command, String> {
    let inherited = std::env::var_os(ENV);
    let leases = crate::retention::active_descriptors()?;
    let mut cmd = if needs_carrier(inherited.is_some(), leases.len()) {
        let current_path = std::env::var_os("PATH").ok_or("PATH is missing")?;
        let search = current_path.to_str().ok_or("non UTF-8 PATH")?;
        let runtime = crate::files::resolve_program(root, "chrono-harness", Some(search))?;
        let mut cmd = Command::new(runtime);
        cmd.arg("exec-with-descriptors-v1").arg(program);
        cmd
    } else {
        Command::new(program)
    };
    cmd.current_dir(root).args(args).env_clear();
    cmd.envs(
        environment
            .iter()
            .filter(|(key, _)| key.as_os_str() != OsStr::new(ENV)),
    );
    if leases.is_empty() {
        if let Some(inherited) = inherited {
            cmd.env(ENV, inherited);
        }
    } else {
        use std::{io, os::fd::AsRawFd, os::unix::process::CommandExt};
        let mut carrier = Vec::new();
        if let Some(inherited) = inherited {
            carrier.push(
                inherited
                    .into_string()
                    .map_err(|_| "non UTF-8 ownership descriptors")?,
            );
        }
        carrier.extend(leases.iter().map(|fd| fd.as_raw_fd().to_string()));
        cmd.env(ENV, carrier.join(","));
        // The runner immediately duplicates and normalizes these child-only carriers.
        unsafe {
            cmd.pre_exec(move || {
                for fd in &leases {
                    rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::empty())
                        .map_err(io::Error::from)?;
                }
                Ok(())
            });
        }
    }
    Ok(cmd)
}

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn needs_carrier(inherited: bool, leases: usize) -> (wrap: bool)
    ensures wrap == (inherited || leases > 0),
{ inherited || leases > 0 }
}
