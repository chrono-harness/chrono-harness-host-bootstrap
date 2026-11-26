//! Bootstrap stream and executable identities use the original file bytes.
#![allow(unexpected_cfgs)]
use vstd::prelude::*;

verus! {
/// Count exactly the bytes written to the output and incorporated into SHA-256.
pub fn add_stream_bytes(total: u64, read: u64) -> (next: Option<u64>)
    ensures match next {
        Some(n) => n as int == total as int + read,
        None => total as int + read > u64::MAX,
    },
{
    if read <= u64::MAX - total { Some(total + read) } else { None }
}
}

#[cfg(not(verus_keep_ghost))]
use sha2::{Digest, Sha256};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub fn sha256(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
#[cfg(not(verus_keep_ghost))]
pub fn copy_hashed(mut input: impl Read, mut output: impl Write) -> Result<(String, u64), String> {
    let mut buffer = [0; 65536];
    let mut hash = Sha256::new();
    let mut total = 0u64;
    loop {
        let n = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hash.update(&buffer[..n]);
        total = add_stream_bytes(total, n as u64).ok_or("stream size overflow")?;
    }
    Ok((hex::encode(hash.finalize()), total))
}
#[cfg(not(verus_keep_ghost))]
pub fn file_identity(path: &Path) -> Result<(String, u64), String> {
    copy_hashed(
        fs::File::open(path).map_err(|e| e.to_string())?,
        std::io::sink(),
    )
}
#[cfg(not(verus_keep_ghost))]
pub fn resolve_program(
    root: &Path,
    program: &str,
    search: Option<&str>,
) -> Result<PathBuf, String> {
    let root = std::path::absolute(root).map_err(|e| e.to_string())?;
    let error = || format!("executable not found: {program}");
    if program.contains('/') {
        let candidate = root.join(program);
        return candidate.is_file().then_some(candidate).ok_or_else(error);
    }
    let path = search
        .map(std::ffi::OsString::from)
        .or_else(|| std::env::var_os("PATH"))
        .ok_or("PATH is missing")?;
    std::env::split_paths(&path)
        .map(|dir| root.join(dir).join(program))
        .find(|candidate| candidate.is_file())
        .ok_or_else(error)
}
#[cfg(not(verus_keep_ghost))]
pub fn identity(path: &Path) -> Result<String, String> {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("ownership identity {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err("ownership lease must be a physical regular file".into());
    }
    Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
}

/// Check the runner wire's lexical relative path before using native host filesystem APIs.
#[cfg(not(verus_keep_ghost))]
pub fn relative_path(relative: &str) -> Result<(), String> {
    if relative.is_empty()
        || relative == "."
        || relative.starts_with("./")
        || relative.starts_with('/')
        || relative.split('/').any(|part| part == "..")
        || relative
            .bytes()
            .any(|b| matches!(b, 0 | b'\n' | b'\r' | b'\\'))
    {
        return Err(format!("invalid relative path: {relative:?}"));
    }
    Ok(())
}
#[cfg(not(verus_keep_ghost))]
pub fn checked_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut path = root.to_owned();
    for part in Path::new(relative).components() {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("symlink path is not allowed: {}", path.display()));
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}
