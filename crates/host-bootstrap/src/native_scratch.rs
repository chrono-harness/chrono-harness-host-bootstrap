//! One lease covers the private compilation directory and the entire native process.
#[cfg(verus_keep_ghost)]
#[path = "native_policy.rs"]
mod native_policy;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::native_policy;
use vstd::prelude::*;

verus! {
enum NativeOutcome { CleanUp, Retain(u8) }

/// Successful children release the scratch directory. Failed children keep their status.
fn native_outcome(code: Option<i32>, signal: Option<i32>) -> (outcome: NativeOutcome)
    ensures match outcome {
        NativeOutcome::CleanUp => code == Some(0),
        NativeOutcome::Retain(exit) => code != Some(0) && exit > 0 && exit as int == match code {
            Some(value) => if 1 <= value <= 255 { value as int } else { 1int },
            None => match signal {
                Some(value) => if 1 <= value <= 127 { 128 + value as int } else { 1int },
                None => 1int,
            },
        },
    },
{
    match native_policy::shell_exit(code, signal) {
        0 => NativeOutcome::CleanUp,
        exit => NativeOutcome::Retain(exit),
    }
}
}

#[cfg(not(verus_keep_ghost))]
use std::collections::HashSet;
#[cfg(not(verus_keep_ghost))]
use std::ffi::{OsStr, OsString};
#[cfg(not(verus_keep_ghost))]
use std::fs::{self, File, OpenOptions};
#[cfg(not(verus_keep_ghost))]
use std::io::{self, ErrorKind};
#[cfg(not(verus_keep_ghost))]
use std::os::unix::ffi::OsStrExt;
#[cfg(not(verus_keep_ghost))]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
#[cfg(not(verus_keep_ghost))]
use std::os::unix::process::{CommandExt, ExitStatusExt};
#[cfg(not(verus_keep_ghost))]
use std::path::{Path, PathBuf};
#[cfg(not(verus_keep_ghost))]
use std::process::Command;
#[cfg(not(verus_keep_ghost))]
use std::time::{Duration, SystemTime};

#[cfg(not(verus_keep_ghost))]
const BUSINESS: &str = "target/native-checks";
#[cfg(not(verus_keep_ghost))]
const PREFIX: &str = "check-";
#[cfg(not(verus_keep_ghost))]
const OWNER: &str = ".chrono-native-owner";

#[cfg(not(verus_keep_ghost))]
fn physical_directory(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::NOFOLLOW).bits() as i32)
        .open(path)
}

#[cfg(not(verus_keep_ghost))]
fn directory(root: &Path, create: bool) -> io::Result<(PathBuf, File)> {
    let mut path = root.to_owned();
    for component in Path::new(BUSINESS).components() {
        path.push(component);
        if create {
            match fs::create_dir(&path) {
                Err(error) if error.kind() != ErrorKind::AlreadyExists => return Err(error),
                _ => {}
            }
        }
        physical_directory(&path)?;
    }
    let gate = physical_directory(&path)?;
    gate.lock()?;
    Ok((path, gate))
}

#[cfg(not(verus_keep_ghost))]
fn identity(metadata: &fs::Metadata) -> String {
    format!("{} {}\n", metadata.dev(), metadata.ino())
}

#[cfg(not(verus_keep_ghost))]
fn tracked(root: &Path, git: &OsStr) -> io::Result<HashSet<OsString>> {
    let out = Command::new(git)
        .current_dir(root)
        .args([
            "ls-files",
            "--cached",
            "--with-tree=HEAD",
            "-z",
            "--",
            BUSINESS,
        ])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git ls-files {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let mut protected = HashSet::new();
    for entry in out.stdout.split(|c| *c == 0).filter(|s| !s.is_empty()) {
        let path = Path::new(OsStr::from_bytes(entry));
        let name = path
            .strip_prefix(BUSINESS)
            .ok()
            .and_then(|p| p.components().next())
            .ok_or_else(|| io::Error::other("unexpected tracked path in native check directory"))?;
        protected.insert(name.as_os_str().to_owned());
    }
    Ok(protected)
}

#[cfg(not(verus_keep_ghost))]
fn dispose(path: &Path, protected: &HashSet<OsString>, age: Option<Duration>) -> io::Result<bool> {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    let generated = name.strip_prefix(PREFIX).is_some_and(|suffix| {
        suffix.len() == 16 && suffix.bytes().all(|c| c.is_ascii_alphanumeric())
    });
    if !native_policy::candidate(generated, protected.contains(OsStr::new(name)))
        || !path.symlink_metadata()?.is_dir()
    {
        return Ok(false);
    }
    let lease = physical_directory(path)?;
    let exclusively_held = match lease.try_lock() {
        Err(std::fs::TryLockError::WouldBlock) => false,
        Err(std::fs::TryLockError::Error(error)) => return Err(error),
        Ok(()) => true,
    };
    if !exclusively_held {
        return Ok(false);
    }
    let stat = lease.metadata()?;
    let keep = age.map(|d| d.as_secs());
    let elapsed = keep
        .and_then(|_| stat.modified().ok())
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .map(|d| d.as_secs());
    if !native_policy::time_ready(keep, elapsed) {
        return Ok(false);
    }
    let marker = path.join(OWNER);
    let owns = match marker.symlink_metadata() {
        Ok(meta) if meta.is_file() && meta.len() <= 64 => {
            let content = fs::read(&marker)?;
            let signature = identity(&stat);
            let complete = content == signature.as_bytes();
            let prefix_only = !complete
                && signature.as_bytes().starts_with(&content)
                && fs::read_dir(path)?
                    .take(2)
                    .collect::<io::Result<Vec<_>>>()?
                    .len()
                    == 1;
            native_policy::marker_owned(complete, prefix_only, false)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            native_policy::marker_owned(false, false, fs::read_dir(path)?.next().is_none())
        }
        Ok(_) => false,
        Err(e) => return Err(e),
    };
    if !native_policy::removable(
        exclusively_held,
        owns,
        identity(&path.symlink_metadata()?) == identity(&stat),
    ) {
        return Ok(false);
    }
    // Remove the ownership marker last so an interrupted sweep can resume.
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_name() == OWNER {
            continue;
        }
        if entry.file_type()?.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    if marker.exists() {
        fs::remove_file(marker)?;
    }
    fs::remove_dir(path)?;
    Ok(true)
}

/// Report a failure to refresh the retained directory without losing the child outcome.
#[cfg(not(verus_keep_ghost))]
pub fn record_failure(
    exit: u8,
    scratch: &Path,
    modified: io::Result<()>,
    diagnostics: &mut impl io::Write,
) -> u8 {
    if let Err(error) = modified {
        let _ = writeln!(
            diagnostics,
            "could not refresh native scratch {}: {error}",
            scratch.display()
        );
    }
    exit
}

/// Run the host's native program with its private scratch path in CHRONO_NATIVE_SCRATCH.
/// The child inherits the shared lock even if its bootstrap parent is interrupted.
#[cfg(not(verus_keep_ghost))]
pub fn run(root: &Path, program: &OsStr, args: &[OsString], git: &OsStr) -> io::Result<u8> {
    let (business, gate) = directory(root, true)?;
    let scratch = tempfile::Builder::new()
        .prefix(PREFIX)
        .rand_bytes(16)
        .tempdir_in(&business)?
        .keep();
    let lease = physical_directory(&scratch)?;
    lease.lock_shared()?;
    fs::write(scratch.join(OWNER), identity(&lease.metadata()?))?;
    gate.unlock()?;
    eprintln!("native check directory: {}", scratch.display());
    let child_lease = lease.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.current_dir(root)
        .args(args)
        .env("CHRONO_NATIVE_SCRATCH", &scratch);
    // Only clear CLOEXEC in the forked child; sibling commands cannot inherit the descriptor.
    unsafe {
        cmd.pre_exec(move || {
            rustix::io::fcntl_setfd(&child_lease, rustix::io::FdFlags::empty())
                .map_err(io::Error::from)
        });
    }
    let status = cmd.status()?;
    drop(cmd);
    if let NativeOutcome::Retain(exit) = native_outcome(status.code(), status.signal()) {
        return Ok(record_failure(
            exit,
            &scratch,
            lease.set_modified(SystemTime::now()),
            &mut io::stderr().lock(),
        ));
    }
    drop(lease);
    gate.lock()?;
    if scratch.exists() {
        dispose(&scratch, &tracked(root, git)?, None)?;
    }
    Ok(0)
}

/// Sweep this host's dedicated native build area from the host's periodic maintenance.
#[cfg(not(verus_keep_ghost))]
pub fn maintain(root: &Path, seconds: u64, git: &OsStr) -> io::Result<usize> {
    if seconds == 0 {
        return Err(io::Error::other("retention must be positive"));
    }
    let (business, _gate) = match directory(root, false) {
        Ok(value) => value,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let tracked = tracked(root, git)?;
    let mut removed = 0;
    for entry in fs::read_dir(business)? {
        if dispose(&entry?.path(), &tracked, Some(Duration::from_secs(seconds)))? {
            removed += 1;
        }
    }
    Ok(removed)
}
