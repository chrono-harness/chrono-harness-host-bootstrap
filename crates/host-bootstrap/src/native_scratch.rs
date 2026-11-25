//! One lease covers the private compilation directory and the entire native process.
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

const BUSINESS: &str = "target/native-checks";
const PREFIX: &str = "check-";
const OWNER: &str = ".chrono-native-owner";

fn physical_directory(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::NOFOLLOW).bits() as i32)
        .open(path)
}

fn directory(root: &Path, create: bool) -> io::Result<(PathBuf, File)> {
    let mut path = root.to_owned();
    for component in Path::new(BUSINESS).components() {
        path.push(component);
        if create
            && let Err(error) = fs::create_dir(&path)
            && error.kind() != ErrorKind::AlreadyExists
        {
            return Err(error);
        }
        physical_directory(&path)?;
    }
    let gate = physical_directory(&path)?;
    gate.lock()?;
    Ok((path, gate))
}

fn identity(metadata: &fs::Metadata) -> String {
    format!("{} {}\n", metadata.dev(), metadata.ino())
}

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

fn dispose(path: &Path, protected: &HashSet<OsString>, age: Option<Duration>) -> io::Result<bool> {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if !name.strip_prefix(PREFIX).is_some_and(|suffix| {
        suffix.len() == 16 && suffix.bytes().all(|c| c.is_ascii_alphanumeric())
    }) || protected.contains(OsStr::new(name))
        || !path.symlink_metadata()?.is_dir()
    {
        return Ok(false);
    }
    let lease = physical_directory(path)?;
    match lease.try_lock() {
        Err(std::fs::TryLockError::WouldBlock) => return Ok(false),
        Err(std::fs::TryLockError::Error(error)) => return Err(error),
        Ok(()) => {}
    }
    let stat = lease.metadata()?;
    if age.is_some_and(|keep| {
        stat.modified()
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_none_or(|elapsed| elapsed < keep)
    }) {
        return Ok(false);
    }
    let marker = path.join(OWNER);
    let owns = match marker.symlink_metadata() {
        Ok(meta) if meta.is_file() && meta.len() <= 64 => {
            let content = fs::read(&marker)?;
            let signature = identity(&stat);
            content == signature.as_bytes()
                || (signature.as_bytes().starts_with(&content)
                    && fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?.len() == 1)
        }
        Err(e) if e.kind() == ErrorKind::NotFound => fs::read_dir(path)?.next().is_none(),
        Ok(_) => false,
        Err(e) => return Err(e),
    };
    if !owns || identity(&path.symlink_metadata()?) != identity(&stat) {
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

/// Run the host's native program with its private scratch path in CHRONO_NATIVE_SCRATCH.
/// The child inherits the shared lock even if its bootstrap parent is interrupted.
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
    if !status.success() {
        lease.set_modified(SystemTime::now())?;
        return Ok(status
            .code()
            .or_else(|| status.signal().map(|s| 128 + s))
            .unwrap_or(1) as u8);
    }
    drop(lease);
    gate.lock()?;
    if scratch.exists() {
        dispose(&scratch, &tracked(root, git)?, None)?;
    }
    Ok(0)
}

/// Sweep this host's dedicated native build area from the host's periodic maintenance.
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
