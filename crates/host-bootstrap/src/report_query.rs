//! Read the producer's original build evidence for release consumers.
#[cfg(verus_keep_ghost)]
use super::artifact_identity;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::artifact_identity;
use vstd::prelude::*;

verus! {
/// An externally requested revision requires both observed Git identities.
pub fn revision_matches(commit: Option<&str>, tree: Option<&str>, requested_commit: &str,
    requested_tree: &str) -> (matched: bool)
    ensures matched == match (commit, tree) {
        (Some(c), Some(t)) => c@ == requested_commit@ && t@ == requested_tree@,
        _ => false,
    },
{
    match (commit, tree) {
        (Some(c), Some(t)) => artifact_identity::equal_text(c, requested_commit)
            && artifact_identity::equal_text(t, requested_tree),
        _ => false,
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{
    bindings::{Environment, NativePlatform, native_platform},
    construction::Report,
    original::Producer,
    profile,
    source::{self, Committed, SourceObservation},
};
#[cfg(not(verus_keep_ghost))]
use serde::Deserialize;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    commit: String,
    tree: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
enum Query {
    Checkout {
        profile: String,
    },
    Retained {
        profile: String,
        platform: NativePlatform,
        revision: Option<Revision>,
        source: SourceObservation,
        report: String,
        directory: PathBuf,
    },
    Streams {
        report: String,
    },
    Source {},
    Platform {},
}

#[cfg(not(verus_keep_ghost))]
fn streams(report: &Report) -> Vec<Value> {
    report
        .operations
        .iter()
        .flat_map(|op| [json!(op.stdout), json!(op.stderr)])
        .collect()
}

#[cfg(not(verus_keep_ghost))]
fn summary(producer: &Producer, report: &Report) -> Value {
    json!({
        "destinations":producer.profile.install.iter().map(|item| &item.destination).collect::<Vec<_>>(),
        "installed":report.installed,
        "streams":streams(report),
    })
}

#[cfg(not(verus_keep_ghost))]
pub fn read(root: &Path, environment: &Environment, input: impl Read) -> Result<Value, String> {
    let query: Query = serde_json::from_reader(input).map_err(|e| e.to_string())?;
    match query {
        Query::Checkout { profile: path } => {
            let native = native_platform()?;
            let source = source::observe(root, environment);
            let producer = Producer::read(&path, &native, &|path| profile::read(root, path, b""))?;
            let raw = profile::read(
                root,
                producer
                    .profile
                    .report_path
                    .as_deref()
                    .ok_or("missing bootstrap report path")?,
                b".chrono-harness/state/",
            )?;
            let report = producer.validate(&raw, &source, &|path| {
                profile::read(root, path, b".chrono-harness/state/")
            })?;
            Ok(
                json!({"source":source,"platform":native,"report":std::str::from_utf8(&raw).map_err(|e|e.to_string())?,
                "summary":summary(&producer,&report)}),
            )
        }
        Query::Retained {
            profile: path,
            platform,
            revision,
            source,
            report,
            directory,
        } => {
            let producer = if let Some(revision) = revision {
                if !revision_matches(
                    source.commit.as_deref(),
                    source.tree.as_deref(),
                    &revision.commit,
                    &revision.tree,
                ) {
                    return Err(
                        "original producer source commit or tree differs from requested revision"
                            .into(),
                    );
                }
                let fixed = Committed::new(root, environment, &revision.commit, &revision.tree)?;
                Producer::read(&path, &platform, &|path| fixed.read(path))?
            } else {
                Producer::read(&path, &platform, &|path| profile::read(root, path, b""))?
            };
            if !directory.is_absolute() {
                return Err("original stream directory must be absolute".into());
            }
            let report = producer.validate(report.as_bytes(), &source, &|path| {
                fs::read(crate::files::checked_path(&directory, path)?).map_err(|e| e.to_string())
            })?;
            Ok(summary(&producer, &report))
        }
        Query::Streams { report } => Ok(json!(streams(&crate::codec::decode(report.as_bytes())?))),
        Query::Source {} => Ok(json!(source::observe(root, environment))),
        Query::Platform {} => Ok(json!(native_platform()?)),
    }
}
