//! owner 原始构建／发布资格的候选读取器；发布和租约仍由现有生命周期所有者执行。
#[cfg(verus_keep_ghost)]
use super::identifiers as stage_identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as stage_identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn stage_reuse_matches(path: &str, digest: &str, adopted_path: &str, adopted_digest: &str) -> (matches: bool)
    ensures matches == (path@ == adopted_path@ && digest@ == adopted_digest@),
{
    stage_identifiers::equal_text(path, adopted_path) && stage_identifiers::equal_text(digest, adopted_digest)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::{
    bindings::{Environment, NativePlatform},
    construction::Report,
    profile::{self, Profile},
    source, verified_bindings as policy,
};
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{decode, file_identity, retained_artifacts::retention, sha256};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
pub const STAGES: &str = ".chrono-harness/bin/owner-stages/";

#[cfg(not(verus_keep_ghost))]
pub fn coordinator_root(
    root: &Path,
    profile: &Profile,
    environment: &Environment,
) -> Result<PathBuf, String> {
    let mut command = crate::bindings::command(
        root,
        Path::new(&profile.participation.git),
        &[
            "--no-optional-locks",
            "worktree",
            "list",
            "--porcelain",
            "-z",
        ],
        environment,
    )?;
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "coordinator Git inventory failed: {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let first = output
        .stdout
        .split(|byte| *byte == 0)
        .next()
        .ok_or("coordinator Git inventory is missing")?;
    let path = first
        .strip_prefix(b"worktree ")
        .ok_or("coordinator Git inventory is missing")?;
    use std::os::unix::ffi::OsStrExt;
    let path = PathBuf::from(OsStr::from_bytes(path));
    if !path.is_absolute() {
        return Err("coordinator Git root must be absolute".into());
    }
    Ok(path)
}

#[cfg(not(verus_keep_ghost))]
pub fn check_shared_install(
    root: &Path,
    environment: &Environment,
    profile: &Profile,
    source: &Path,
    target: &Path,
) -> Result<(), String> {
    if environment
        .get(OsStr::new("CHRONO_WORKTREE_BOOTSTRAP"))
        .map(|value| Path::new(value))
        != Some(root)
    {
        return Ok(());
    }
    let anchor = coordinator_root(root, profile, environment)?;
    let local = profile::path(
        root,
        &profile.participation.program,
        b".chrono-harness/bin/",
    )?;
    if target != local {
        return Err("lifecycle install destination differs from participation".into());
    }
    let current = if target.exists() {
        Some(file_binding(target)?)
    } else {
        None
    };
    if anchor != root {
        let selected = profile::path(
            &anchor,
            &profile.participation.program,
            b".chrono-harness/bin/",
        )?;
        let coordinator = file_binding(&selected)?;
        if current
            .as_ref()
            .is_none_or(|binding| binding.identity != coordinator.identity)
        {
            return Ok(());
        }
    }
    let Some(current) = current else {
        return Err("changed lifecycle executable requires exclusive owner deployment".into());
    };
    if file_binding(source)?.sha256 != current.sha256 {
        return Err("changed lifecycle executable requires exclusive owner deployment".into());
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
pub struct Reuse {
    pub operations: BTreeMap<String, crate::execution::Operation>,
    pub source: Option<PathBuf>,
}

/// 原操作只沿用当前 adoption 指向的原始构建，保留 PID、退出和两流，不伪造第二次执行。
#[cfg(not(verus_keep_ghost))]
pub fn reuse_construction(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    profile: &Profile,
    result: &Report,
    observed: &source::SourceObservation,
) -> Result<Reuse, String> {
    let empty = || Reuse {
        operations: BTreeMap::new(),
        source: None,
    };
    if profile.reuse_owner_operations.is_empty()
        || coordinator_root(root, profile, environment)? != root
    {
        return Ok(empty());
    }
    let Some(adoption) = adopted_owner(
        root,
        environment,
        &profile.participation.program,
        false,
        None,
    )?
    else {
        return Ok(empty());
    };
    if Some(adoption.source_commit.as_str()) != observed.commit.as_deref() {
        return Err("owner construction must belong to the current candidate".into());
    }
    let qualification = qualified_publication(
        root,
        environment,
        native,
        &adoption,
        &profile.participation.program,
        profile
            .report_path
            .as_deref()
            .ok_or("owner construction requires a current report path")?,
    )?;
    let qualified = stage_report(
        root,
        environment,
        native,
        &qualification.path,
        Some(&qualification.sha256),
        None,
        qualification.report_slot.as_deref(),
    )?;
    let previous = serde_json::to_value(&qualified.profile).map_err(|error| error.to_string())?;
    let candidate = serde_json::to_value(profile).map_err(|error| error.to_string())?;
    for field in [
        "projects",
        "rust_toolchain",
        "rust_incremental",
        "require_tool_bindings",
        "tool_bindings_registry",
        "host_environment",
        "environment",
    ] {
        if previous.get(field) != candidate.get(field) {
            return Err(format!("owner construction selection differs: {field}"));
        }
    }
    let original = serde_json::to_value(&qualified.report).map_err(|error| error.to_string())?;
    let actual = serde_json::to_value(result).map_err(|error| error.to_string())?;
    for field in [
        "environment",
        "executables",
        "selection",
        "versions",
        "host_environment",
    ] {
        if original.get(field) != actual.get(field) {
            return Err(format!("owner construction observation differs: {field}"));
        }
    }
    let mut rows: BTreeMap<_, _> = qualified
        .report
        .operations
        .into_iter()
        .map(|row| (row.operation.clone(), row))
        .collect();
    let mut reused = BTreeMap::new();
    for name in &profile.reuse_owner_operations {
        let mut row = rows
            .remove(name)
            .ok_or("owner construction is missing a declared operation")?;
        if qualified.profile.tools.get(&row.tool) != profile.tools.get(&row.tool) {
            return Err(format!(
                "owner construction selection differs: tool {}",
                row.tool
            ));
        }
        row.reused_from = Some(crate::execution::StreamOrigin {
            path: qualification.path.clone(),
            sha256: qualification.sha256.clone(),
        });
        reused.insert(name.clone(), row);
    }
    let source = profile::path(root, &qualified.stage.staged.path, STAGES.as_bytes())?;
    Ok(Reuse {
        operations: reused,
        source: Some(source),
    })
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg(not(verus_keep_ghost))]
pub struct Reference {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize)]
#[cfg(not(verus_keep_ghost))]
pub struct ProducerSource {
    pub commit: String,
    pub tree: String,
}

#[derive(Debug, Serialize)]
#[cfg(not(verus_keep_ghost))]
pub struct Qualification {
    pub schema: &'static str,
    pub path: String,
    pub sha256: String,
    pub source: ProducerSource,
    pub selected: Binding,
    #[serde(skip)]
    pub(crate) report_slot: Option<String>,
}

/// Executable observation recorded by the bootstrap stage producer.
#[cfg(not(verus_keep_ghost))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub identity: String,
    pub sha256: String,
    pub size: u64,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Staged {
    pub path: String,
    identity: String,
    sha256: String,
    size: u64,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    selected: String,
    previous: Binding,
    pub staged: Staged,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Adoption {
    source_commit: String,
    stage_result: String,
    stage_sha256: String,
    installed: Binding,
    stage: Stage,
    report_slot: String,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn adopted_owner(
    root: &Path,
    environment: &Environment,
    selected: &str,
    if_installed: bool,
    commit: Option<&str>,
) -> Result<Option<Adoption>, String> {
    let path = environment
        .get(OsStr::new("PATH"))
        .map(|value| value.to_str().ok_or("non UTF-8 PATH"))
        .transpose()?;
    let program = chrono_harness::resolve_program(root, "chrono-worktree", path)?;
    let mut args = vec![
        "adopted-owner-v1",
        root.to_str().ok_or("non UTF-8 host root")?,
        selected,
    ];
    if if_installed {
        args.push("--if-installed");
        args.extend(commit);
    }
    let output = crate::bindings::capture(root, &program, &args, environment)?;
    if !output.status.success() {
        return Err(format!(
            "owner adoption query {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    decode(&output.stdout)
}

#[cfg(not(verus_keep_ghost))]
pub struct QualifiedStage {
    pub report: Report,
    pub profile: Profile,
    pub stage: Stage,
    pub raw: Vec<u8>,
}

#[cfg(not(verus_keep_ghost))]
pub fn file_binding(path: &Path) -> Result<Binding, String> {
    let identity = chrono_harness::ownership::identity(path)?;
    let before = fs::metadata(path).map_err(|error| error.to_string())?;
    let (digest, size) = file_identity(path)?;
    let after = fs::metadata(path).map_err(|error| error.to_string())?;
    let stamp = |metadata: &fs::Metadata| (metadata.len(), metadata.mtime(), metadata.mtime_nsec());
    if identity != chrono_harness::ownership::identity(path)?
        || stamp(&before) != stamp(&after)
        || size != after.len()
    {
        return Err("owner executable changed during observation".into());
    }
    Ok(Binding {
        identity,
        sha256: digest,
        size,
    })
}

#[cfg(not(verus_keep_ghost))]
pub fn require_capabilities(root: &Path, environment: &Environment) -> Result<(), String> {
    if environment
        .get(OsStr::new("CHRONO_WORKTREE_BOOTSTRAP"))
        .map(|value| Path::new(value))
        != Some(root)
        || !chrono_harness::process_fds::inherited_available()?
    {
        return Err(
            "owner operation requires original ownership descriptors and host binding".into(),
        );
    }
    Ok(())
}

/// 只构造独立 stage，不改选中 owner；独占发布仍由原 Rust 生命周期入口执行。
#[cfg(not(verus_keep_ghost))]
pub fn stage(
    root: &Path,
    environment: &Environment,
    profile: &Profile,
    previous: &Binding,
    report_directory: &str,
) -> Result<Stage, String> {
    require_capabilities(root, environment)?;
    let declaration = profile
        .owner_stage
        .as_ref()
        .ok_or("owner stage declaration is missing")?;
    let selected = profile::path(root, &declaration.selected, b".chrono-harness/bin/")?;
    if file_binding(&selected)? != *previous {
        return Err("selected owner changed during construction".into());
    }
    let source = profile::path(root, &declaration.source, b"")?;
    let before = file_binding(&source)?;
    let parent = profile::path(root, STAGES.trim_end_matches('/'), b".chrono-harness/bin/")?;
    fs::create_dir_all(&parent).map_err(|error| error.to_string())?;
    let directory = retention::unique_path(&parent, "stage-", "")?;
    let publication = retention::create_adopted(
        root,
        "bootstrap-owner-stage",
        &directory,
        true,
        serde_json::json!({"stage":"construction","publication":"not-established"}),
    )?;
    let relative = directory
        .strip_prefix(root)
        .map_err(|e| e.to_string())?
        .to_str()
        .ok_or("non UTF-8 owner stage")?;
    if publication.is_some() {
        let inventory =
            retention::Inventory::adopted(root)?.ok_or("owner stage policy disappeared")?;
        inventory.retain_diagnostic_path(relative)?;
        // The live construction report owns this stage before any writer can
        // release it. Current/adopted report references later own its lifetime.
        inventory.dependencies_paths(report_directory, &[relative.into()])?;
    }
    let target = directory.join("chrono-worktree");
    retention::copy_adopted(publication.as_ref(), &source, &target)?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    let staged = file_binding(&target)?;
    if file_binding(&source)? != before
        || staged.sha256 != before.sha256
        || staged.size != before.size
    {
        return Err("built owner changed during staging".into());
    }
    Ok(Stage {
        selected: declaration.selected.clone(),
        previous: previous.clone(),
        staged: Staged {
            path: target
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or("non UTF-8 owner stage")?
                .into(),
            identity: staged.identity,
            sha256: staged.sha256,
            size: staged.size,
        },
    })
}

#[cfg(not(verus_keep_ghost))]
pub fn stage_report(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    reference: &str,
    expected_sha: Option<&str>,
    producer_commit: Option<&str>,
    report_slot: Option<&str>,
) -> Result<QualifiedStage, String> {
    let raw = profile::read_original(
        root,
        reference,
        b".chrono-harness/state/bootstrap-",
        report_slot,
    )?;
    if let Some(expected) = expected_sha {
        if !chrono_harness::verified_identifiers::equal_text(&sha256(&raw), expected) {
            return Err("owner stage original report differs".into());
        }
    }
    let decoded: Report = decode(&raw)?;
    if Path::new(&decoded.host_root) != root {
        return Err("owner stage producer belongs to another host".into());
    }
    let fixed;
    let current;
    let input: Box<dyn Fn(&str) -> Result<Vec<u8>, String> + '_>;
    let observed;
    if let Some(expected) = producer_commit {
        let identity = decoded
            .source
            .as_ref()
            .ok_or("owner stage source is missing")?;
        let commit = identity
            .commit
            .as_deref()
            .ok_or("owner stage commit is missing")?;
        if commit != expected {
            return Err("selected owner stage producer differs".into());
        }
        fixed = source::Committed::new(
            root,
            environment,
            commit,
            identity
                .tree
                .as_deref()
                .ok_or("owner stage tree is missing")?,
        )?;
        observed = fixed.observation();
        input = Box::new(|path| fixed.read(path));
    } else {
        current = source::observe(root, environment);
        observed = current;
        input = Box::new(|path| profile::read(root, path, b".chrono-harness/"));
    }
    let producer = crate::original::Producer::read(&decoded.config, native, &input)?;
    let report = producer.validate(&raw, &observed, &|path| {
        profile::read_original(root, path, b".chrono-harness/state/", report_slot)
    })?;
    let stage: Stage = serde_json::from_value(
        report
            .owner_stage
            .clone()
            .ok_or("owner stage record is missing")?,
    )
    .map_err(|error| error.to_string())?;
    let declared = producer.profile.owner_stage.as_ref();
    if !policy::owner_stage_input(
        report.installed.is_empty(),
        declared.is_some(),
        &stage.selected,
        declared.map(|item| item.selected.as_str()).unwrap_or(""),
    ) || !policy::host_path(stage.selected.as_bytes(), b".chrono-harness/bin/")
    {
        return Err("owner stage declaration differs".into());
    }
    let staged = profile::path(root, &stage.staged.path, STAGES.as_bytes())?;
    let actual = file_binding(&staged)?;
    if actual.identity != stage.staged.identity
        || actual.sha256 != stage.staged.sha256
        || actual.size != stage.staged.size
    {
        return Err("retained owner stage changed".into());
    }
    Ok(QualifiedStage {
        report,
        profile: producer.profile,
        stage,
        raw,
    })
}

#[cfg(not(verus_keep_ghost))]
fn qualification(
    reference: &str,
    raw: &[u8],
    report: &Report,
    selected: Binding,
    report_slot: Option<&str>,
) -> Result<Qualification, String> {
    let identity = report
        .source
        .as_ref()
        .ok_or("owner producer source is missing")?;
    Ok(Qualification {
        schema: "chrono-bootstrap-selected-original/v1",
        path: reference.into(),
        sha256: sha256(raw),
        source: ProducerSource {
            commit: identity
                .commit
                .as_ref()
                .ok_or("owner producer commit is missing")?
                .clone(),
            tree: identity
                .tree
                .as_ref()
                .ok_or("owner producer tree is missing")?
                .clone(),
        },
        selected,
        report_slot: report_slot.map(str::to_owned),
    })
}

// Older adoptions could name a rotating slot. The installed report's reused
// operations record the original stream paths and the exact stage digest.
#[cfg(not(verus_keep_ghost))]
fn adopted_stage_source<'a>(
    root: &Path,
    adoption: &'a Adoption,
    installation_report: &'a str,
) -> Result<(String, &'a str), String> {
    let stage = Path::new(&adoption.stage_result);
    let state = Path::new(".chrono-harness/state");
    if stage.parent() != Some(state) {
        return Ok((adoption.stage_result.clone(), &adoption.report_slot));
    }
    let fixed_slot = profile::path(
        root,
        &adoption.stage_result,
        b".chrono-harness/state/bootstrap-",
    )?;
    if fixed_slot.try_exists().map_err(|e| e.to_string())?
        && sha256(&profile::read_original(
            root,
            &adoption.stage_result,
            b".chrono-harness/state/bootstrap-",
            Some(&adoption.stage_result),
        )?) == adoption.stage_sha256
    {
        return Ok((adoption.stage_result.clone(), &adoption.stage_result));
    }
    let report: Report = decode(&profile::read_original(
        root,
        installation_report,
        b".chrono-harness/state/",
        Some(installation_report),
    )?)?;
    if Path::new(&report.host_root) != root
        || !matches!(report.status, policy::BuildStatus::Passed)
        || report.source.as_ref().and_then(|s| s.commit.as_deref()) != Some(&adoption.source_commit)
    {
        return Err("installed report does not belong to the adopted stage".into());
    }
    let mut directories = BTreeSet::new();
    for row in &report.operations {
        if row.reused_from.as_ref().is_some_and(|source| {
            stage_reuse_matches(
                &source.path,
                &source.sha256,
                &adoption.stage_result,
                &adoption.stage_sha256,
            )
        }) {
            for stream in [&row.stdout, &row.stderr] {
                let (directory, _) = stream
                    .path
                    .rsplit_once('/')
                    .ok_or("stage stream directory missing")?;
                if Path::new(directory).parent() != Some(state) {
                    return Err("invalid stage stream directory".into());
                }
                profile::path(root, &stream.path, b".chrono-harness/state/bootstrap-")?;
                directories.insert(directory);
            }
        }
    }
    if directories.len() != 1 {
        return Err("installed stage stream directory missing or ambiguous".into());
    }
    Ok((
        format!("{}/report.json", directories.first().unwrap()),
        installation_report,
    ))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn qualified_publication(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    adoption: &Adoption,
    selected: &str,
    installation_report: &str,
) -> Result<Qualification, String> {
    let (reference, report_slot) = adopted_stage_source(root, adoption, installation_report)?;
    let qualified = stage_report(
        root,
        environment,
        native,
        &reference,
        Some(&adoption.stage_sha256),
        Some(&adoption.source_commit),
        Some(report_slot),
    )?;
    if reference != adoption.stage_result
        && qualified.profile.report_path.as_deref() != Some(adoption.stage_result.as_str())
    {
        return Err("adopted slot differs from the original stage profile".into());
    }
    let record = &qualified.stage;
    let selected_path = profile::path(root, selected, b".chrono-harness/bin/")?;
    let installed = file_binding(&selected_path)?;
    if record.selected != selected || installed != adoption.installed || *record != adoption.stage {
        return Err("selected owner publication originals differ".into());
    }
    qualification(
        &reference,
        &qualified.raw,
        &qualified.report,
        installed,
        Some(report_slot),
    )
}

#[cfg(not(verus_keep_ghost))]
pub fn qualified_selected_original(
    root: &Path,
    environment: &Environment,
    native: &NativePlatform,
    reference: &str,
    selected: &str,
) -> Result<Qualification, String> {
    let selected_path = profile::path(root, selected, b".chrono-harness/bin/")?;
    let before = file_binding(&selected_path)?;
    if let Some(adoption) = adopted_owner(root, environment, selected, true, None)? {
        return qualified_publication(root, environment, native, &adoption, selected, reference);
    }
    let raw = profile::read(root, reference, b".chrono-harness/state/")?;
    let report = crate::original::validate_fixed_execution(root, environment, native, &raw)?;
    let identity = report
        .source
        .as_ref()
        .ok_or("selected owner source is missing")?;
    let fixed = source::Committed::new(
        root,
        environment,
        identity
            .commit
            .as_deref()
            .ok_or("selected owner commit is missing")?,
        identity
            .tree
            .as_deref()
            .ok_or("selected owner tree is missing")?,
    )?;
    let config = Profile::parse(&fixed.read(&report.config)?)?;
    if config.report_path.as_deref() != Some(reference)
        || !policy::unique_method(
            config
                .install
                .iter()
                .filter(|item| item.destination == selected)
                .count(),
        )
    {
        return Err("selected owner original producer destination differs".into());
    }
    let expected_paths = config
        .install
        .iter()
        .map(|item| item.destination.clone())
        .collect();
    let installed_paths = report
        .installed
        .iter()
        .map(|item| item.path.clone())
        .collect();
    if !policy::text_list_equal(&installed_paths, &expected_paths) {
        return Err("selected owner original installed set differs".into());
    }
    for installed in &report.installed {
        let actual = file_binding(&profile::path(
            root,
            &installed.path,
            b".chrono-harness/bin/",
        )?)?;
        if actual.sha256 != installed.sha256 {
            return Err("selected owner original installed bytes differ".into());
        }
    }
    if profile::read(root, reference, b".chrono-harness/state/")? != raw
        || file_binding(&selected_path)? != before
    {
        return Err("selected owner or its original changed during qualification".into());
    }
    qualification(reference, &raw, &report, before, None)
}
