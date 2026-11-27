//! 固定归档的本地复用；摘要来自现役声明，网络与进程仍用已有工具。
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use super::*;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::{archive_matches, byte_prefix, process_exit, valid_sha256};
#[cfg(not(verus_keep_ghost))]
use crate::{
    bindings::{self, Environment},
    construction::BuildFailure,
    files::file_identity,
    profile,
};
#[cfg(not(verus_keep_ghost))]
use serde::Serialize;
#[cfg(not(verus_keep_ghost))]
use std::{fs, io, os::unix::process::ExitStatusExt, path::Path};

verus! {
pub fn cached_archive<E>(observed: Result<Option<&[u8]>, E>, expected: &[u8])
    -> (out: Result<bool, E>)
    ensures match observed {
        Err(error) => out == Err::<bool, E>(error),
        Ok(None) => out == Ok::<bool, E>(false),
        Ok(Some(digest)) => out == Ok::<bool, E>(expected@.len() == 64
            && (forall|i: int| 0 <= i < expected@.len() ==> hash_byte(expected@[i]))
            && digest@ == expected@),
    },
{
    match observed {
        Err(error) => Err(error),
        Ok(None) => Ok(false),
        Ok(Some(digest)) => Ok(archive_matches(digest, expected)),
    }
}

pub fn check_download(digest: &[u8], expected: &[u8]) -> (out: Result<(), &'static str>)
    ensures out.is_ok() == (expected@.len() == 64
        && (forall|i: int| 0 <= i < expected@.len() ==> hash_byte(expected@[i]))
        && digest@ == expected@),
        out.is_err() ==> out.unwrap_err()@ == "downloaded archive checksum mismatch"@,
{
    if archive_matches(digest, expected) {
        Ok(())
    } else {
        Err("downloaded archive checksum mismatch")
    }
}

pub fn valid_declaration(expected: &[u8], url: &[u8], transport_prefix: &[u8]) -> (valid: bool)
    ensures valid == (expected@.len() == 64
        && (forall|i: int| 0 <= i < expected@.len() ==> hash_byte(expected@[i]))
        && transport_prefix@.len() <= url@.len()
        && (forall|i: int| 0 <= i < transport_prefix@.len() ==> url@[i] == transport_prefix@[i])),
{
    valid_sha256(expected) && byte_prefix(url, transport_prefix)
}
}

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Serialize)]
pub struct Archive {
    pub status: &'static str,
    pub path: String,
    pub sha256: String,
}

#[cfg(not(verus_keep_ghost))]
pub fn prepare(
    root: &Path,
    checksum: &str,
    curl: &Path,
    headers: &str,
    url: &str,
    environment: &Environment,
) -> Result<Archive, BuildFailure> {
    let declaration = profile::read(root, checksum, b".chrono-harness/ci/")?;
    let text = std::str::from_utf8(&declaration).map_err(|error| error.to_string())?;
    let (expected, archive) = text
        .strip_suffix('\n')
        .unwrap_or(text)
        .split_once("  ")
        .ok_or("archive checksum requires one SHA256 and path record")?;
    if !valid_declaration(expected.as_bytes(), url.as_bytes(), b"https://") {
        return Err("invalid archive checksum or HTTPS URL".into());
    }
    let destination = profile::path(root, archive, b".chrono-harness/cache/")?;
    let header_path = profile::path(root, headers, b".chrono-harness/state/")?;
    let existing = match fs::symlink_metadata(&destination) {
        Ok(_) => file_identity(&destination).map(|(digest, _)| Some(digest)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    };
    let observed = existing
        .as_ref()
        .map(|digest| digest.as_deref().map(str::as_bytes));
    if cached_archive(observed, expected.as_bytes()).map_err(String::from)? {
        return Ok(Archive {
            status: "cache-hit",
            path: archive.into(),
            sha256: expected.into(),
        });
    }
    let parent = destination.parent().ok_or("missing archive parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::create_dir_all(header_path.parent().ok_or("missing headers parent")?)
        .map_err(|error| error.to_string())?;
    // 正式 profile 将 TMPDIR 交给已有可回收目录；保留包目录不收容中断残留。
    let temporary = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
    let output = temporary
        .path()
        .to_str()
        .ok_or("non UTF-8 archive temporary path")?;
    let headers = header_path.to_str().ok_or("non UTF-8 headers path")?;
    let status = bindings::command(
        root,
        curl,
        &[
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--header",
            "Accept: application/octet-stream",
            "--dump-header",
            headers,
            "--output",
            output,
            url,
        ],
        environment,
    )?
    .status()
    .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(BuildFailure {
            message: format!("archive download failed: {status}"),
            exit_code: process_exit(status.code(), status.signal()),
        });
    }
    let digest = file_identity(temporary.path())?.0;
    check_download(digest.as_bytes(), expected.as_bytes()).map_err(String::from)?;
    temporary
        .persist(&destination)
        .map_err(|error| error.to_string())?;
    Ok(Archive {
        status: "downloaded",
        path: archive.into(),
        sha256: digest,
    })
}
