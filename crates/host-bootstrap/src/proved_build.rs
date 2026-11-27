//! 正式构建消费官方 Cargo-Verus 的原输出、编译器 checksum 和原库字节。
#[cfg(not(verus_keep_ghost))]
use crate::compiler::{self, OutputContract};
#[cfg(not(verus_keep_ghost))]
use crate::files::file_identity;
#[cfg(not(verus_keep_ghost))]
use crate::retention;
#[cfg(not(verus_keep_ghost))]
use crate::verified_cargo::{
    self as cargo, Artifact, BuildTargets, Cargo, Identity, Record, Recovery, TargetIdentity,
};
#[cfg(not(verus_keep_ghost))]
use crate::{bindings::Environment, construction::BuildFailure, execution, profile};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Library {
    pub manifest: String,
    pub source: String,
    pub target: String,
    pub sources: Vec<String>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binary {
    pub source: String,
    pub target: String,
    pub published: String,
    pub dependency_prefix: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    pub owner: String,
    pub operation: String,
    pub projects: String,
    pub recovery_operation: String,
    pub manifest: String,
    pub target: String,
    pub cargo_verus: String,
    pub verus_directory: String,
    pub rust_toolchain: String,
    pub output_contract: OutputContract,
    pub library: Library,
    pub binary: Binary,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    schema: String,
    inputs: String,
    operation: execution::Operation,
    stdout: String,
    stderr: Vec<u8>,
    artifact: Artifact,
    libraries: Vec<Identity>,
    dependency: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Serialize)]
struct Admission {
    schema: &'static str,
    proof_cache: Identity,
    library: Vec<Identity>,
    binary: Identity,
    proof_reused: bool,
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !cargo::bounded_original(metadata.file_type().is_file(), metadata.len()) {
        return Err(format!(
            "E_VERUS_BUILD: original exceeds its registered bound: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(8_388_609)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !cargo::bounded_original(true, bytes.len() as u64) {
        return Err(format!(
            "E_VERUS_BUILD: original exceeds its registered bound: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

#[cfg(not(verus_keep_ghost))]
fn identity(path: &Path) -> Result<Identity, String> {
    let (sha256, length) = file_identity(path)?;
    Ok(Identity {
        path: path.to_str().ok_or("non UTF-8 compiler artifact")?.into(),
        sha256,
        length,
    })
}

#[cfg(not(verus_keep_ghost))]
fn preserve_if_present(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::symlink_metadata(source) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
        Ok(_) => fs::write(destination, bounded(source)?).map_err(|e| e.to_string()),
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn persist(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().ok_or("missing proof parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer(temporary.as_file_mut(), value).map_err(|e| e.to_string())?;
    temporary.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn records(raw: &[u8], contract: &OutputContract) -> Result<Vec<Record>, String> {
    let rows: Vec<&serde_json::value::RawValue> = serde_json::Deserializer::from_slice(raw)
        .into_iter()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("E_VERUS_BUILD: complete Cargo stdout rejected: {e}"))?;
    let texts: Vec<_> = rows.iter().map(|row| row.get()).collect();
    let accepted = compiler::proof_rows(contract, &texts)?;
    rows.into_iter()
        .zip(accepted)
        .map(|(row, result)| match result {
            Some(valid) => Ok(Record::Proof(valid)),
            None => serde_json::from_str::<Cargo>(row.get())
                .map(Record::Cargo)
                .map_err(|e| format!("E_VERUS_BUILD: complete Cargo stdout rejected: {e}")),
        })
        .collect()
}

#[cfg(not(verus_keep_ghost))]
fn selected(artifact: &Artifact, manifest: &Path, source: &Path, name: &str, kind: &str) -> bool {
    cargo::artifact_selected(
        artifact,
        &manifest.to_string_lossy(),
        &source.to_string_lossy(),
        name,
        kind,
    )
}

#[cfg(not(verus_keep_ghost))]
fn library_paths(
    root: &Path,
    config: &Config,
    artifact: &Artifact,
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let directory = profile::path(root, &config.target, b"")?.join("debug/deps");
    let paths = cargo::compiler_library_paths(
        directory.as_os_str().as_encoded_bytes(),
        config.library.target.as_bytes(),
        &artifact.filenames,
        artifact.executable.is_some(),
    )
    .ok_or("E_VERUS_BUILD: compiler library filenames do not match the registered target")?;
    let path = |bytes: &[u8]| -> Result<PathBuf, String> {
        Ok(PathBuf::from(
            std::str::from_utf8(bytes).map_err(|e| e.to_string())?,
        ))
    };
    Ok((
        path(&paths.dependency)?,
        paths
            .outputs
            .iter()
            .map(|bytes| path(bytes))
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

#[cfg(not(verus_keep_ghost))]
pub(crate) enum SourcePaths {
    Literal,
    Resolved,
}

#[cfg(not(verus_keep_ghost))]
impl SourcePaths {
    fn identity(&self, path: &Path) -> Result<PathBuf, String> {
        match self {
            Self::Literal => Ok(path.to_owned()),
            Self::Resolved => fs::canonicalize(path).map_err(|e| e.to_string()),
        }
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn bind_dependency(
    manifest: &Path,
    dependency: &Path,
    outputs: &[PathBuf],
    required: &[PathBuf],
    names: SourcePaths,
) -> Result<Vec<u8>, String> {
    let raw = bounded(dependency)?;
    let stem = dependency.with_extension("");
    let paths = compiler::compiler::<Option<Vec<Vec<u8>>>>(json!({"operation":"dependency-paths","dependency":raw,"stem":stem.as_os_str().as_encoded_bytes()}))?
        .ok_or("E_VERUS_BUILD: compiler dependency header rejected")?;
    let mut bindings = Vec::new();
    let required = required
        .iter()
        .map(|path| names.identity(path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = vec![false; required.len()];
    for path in paths {
        let text = std::str::from_utf8(&path).map_err(|e| e.to_string())?;
        let absolute = if Path::new(text).is_absolute() {
            PathBuf::from(text)
        } else {
            manifest
                .parent()
                .ok_or("missing compiler working directory")?
                .join(text)
        };
        let observed = identity(&absolute)?;
        let actual = names.identity(&absolute)?;
        for (index, expected) in required.iter().enumerate() {
            if &actual == expected {
                seen[index] = true;
            }
        }
        bindings.push(
            json!({"path":path,"sha256":observed.sha256.into_bytes(),"length":observed.length}),
        );
    }
    let mut targets = vec![dependency.as_os_str().as_encoded_bytes().to_vec()];
    targets.extend(
        outputs
            .iter()
            .map(|p| p.as_os_str().as_encoded_bytes().to_vec()),
    );
    if !cargo::all_present(&seen)
        || !compiler::compiler::<bool>(
            json!({"operation":"dependency-matches","dependency":raw,"targets":targets,"sources":bindings}),
        )?
    {
        return Err("E_VERUS_BUILD: compiler source or binary checksums differ".into());
    }
    Ok(raw)
}

/// Admit the original compiler dependency information supplied by an independent owner check.
#[cfg(not(verus_keep_ghost))]
pub fn check_owner_sources(
    kind: &str,
    output: &Path,
    sources: &[String],
    query: impl Fn(serde_json::Value) -> Result<serde_json::Value, String>,
) -> Result<(), String> {
    let mode = owner_source_mode(kind, sources.len()).ok_or("no source files declared")?;
    let mut dependency = output.as_os_str().to_os_string();
    dependency.push(".d");
    let dependency = PathBuf::from(dependency);
    let raw = bounded(&dependency)?;
    let matched = match mode {
        OwnerSourceMode::Native => {
            use std::os::unix::ffi::OsStringExt;
            let result = query(json!({"operation":"dependency-header", "dependency":raw}))?;
            let header: Option<(Vec<u8>, Vec<Vec<u8>>)> =
                serde_json::from_value(result).map_err(|e| e.to_string())?;
            let (stem, paths) = header.ok_or("invalid native compiler dependency header")?;
            let mut declared = std::ffi::OsString::from_vec(stem);
            declared.push(".d");
            let declared = Path::new(&declared);
            let lexical = declared.cmp(dependency.as_path());
            if matches!(native_header(lexical, None), NativeHeader::InspectAlias) {
                let original = fs::symlink_metadata(declared).map_err(|e| e.to_string())?;
                let selected = fs::canonicalize(&dependency).map_err(|e| e.to_string())?;
                let actual = fs::canonicalize(declared).map_err(|e| e.to_string())?;
                let observation = (original.file_type().is_file(), actual.cmp(&selected));
                if matches!(
                    native_header(lexical, Some(observation)),
                    NativeHeader::Reject
                ) {
                    return Err("native compiler header names another dependency file".into());
                }
            }
            required_compiler_paths(&paths, sources)
        }
        OwnerSourceMode::Proof => {
            let bindings = sources
                .iter()
                .map(|name| {
                    let (digest, length) = file_identity(Path::new(name))?;
                    Ok(json!({"path":name.as_bytes(),"sha256":digest.as_bytes(),"length":length}))
                })
                .collect::<Result<Vec<_>, String>>()?;
            serde_json::from_value(query(json!({"operation":"dependency-matches","dependency":raw,
                "targets":[dependency.as_os_str().as_encoded_bytes(),output.as_os_str().as_encoded_bytes()],
                "sources":bindings}))?).map_err(|e| e.to_string())?
        }
        OwnerSourceMode::Unknown => {
            return Err(format!("unknown compiler output kind {kind:?}"));
        }
    };
    if matched {
        Ok(())
    } else {
        Err(format!(
            "{kind} compiler sources differ for {}",
            output.display()
        ))
    }
}

/// Collect the original result of a required owner operation after execution.
#[cfg(not(verus_keep_ghost))]
pub fn check_owner_proof(
    contract: &OutputContract,
    report: &Path,
    output: &Path,
    sources: &[String],
    query: impl Fn(serde_json::Value) -> Result<serde_json::Value, String>,
) -> Result<String, String> {
    let original = String::from_utf8(bounded(report)?).map_err(|e| e.to_string())?;
    let artifact = fs::symlink_metadata(output).map_err(|e| e.to_string())?;
    if !cargo::nonempty_artifact(artifact.file_type().is_file(), artifact.len()) {
        return Err(format!("missing compiler library: {}", output.display()));
    }
    let verdict = query(json!({"operation":"proof-rows","contract":contract,"rows":[&original]}))?;
    let rows: Vec<Option<bool>> = serde_json::from_value(verdict).map_err(|e| e.to_string())?;
    if !cargo::one_successful_proof(&rows) {
        return Err(format!("unexpected verifier report: {}", report.display()));
    }
    check_owner_sources("proof", output, sources, query)?;
    Ok(original)
}

#[cfg(not(verus_keep_ghost))]
fn stream_matches(root: &Path, stream: &execution::Stream, bytes: &[u8]) -> Result<(), String> {
    if !cargo::content_matches(
        &crate::files::sha256(bytes),
        bytes.len() as u64,
        &stream.sha256,
        stream.size,
    ) {
        return Err("E_VERUS_BUILD: original stream identity differs".into());
    }
    profile::path(root, &stream.path, b".chrono-harness/state/")?;
    Ok(())
}

/// 官方构建进程仍由既有 runner 执行；这里不实现编译器 wrapper。
#[cfg(not(verus_keep_ghost))]
pub fn build(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
) -> Result<(), BuildFailure> {
    build_attempt(root, declaration, inherited, stdout, stderr, true)
}

#[cfg(not(verus_keep_ghost))]
fn build_attempt(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
    can_recover: bool,
) -> Result<(), BuildFailure> {
    let parent = profile::path(
        root,
        ".chrono-harness/state/proved-builds",
        b".chrono-harness/state/",
    )?;
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new()
        .prefix("build-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let result = run_attempt(
        root,
        declaration,
        inherited,
        stdout,
        stderr,
        can_recover,
        temporary.path(),
    );
    let rejected = temporary
        .path()
        .join("cache-rejected.json")
        .try_exists()
        .unwrap_or(true);
    if cargo::retain_attempt(result.is_ok(), rejected) {
        let directory = temporary.keep();
        let retained = (|| -> Result<(), String> {
            let relative = directory
                .strip_prefix(root)
                .map_err(|e| e.to_string())?
                .to_str()
                .ok_or("non UTF-8 original directory")?;
            let consumer = format!("proved-build-repair:{relative}");
            let outcome = serde_json::json!({"owner":"registered build caller", "consumer":consumer,
                "status":if result.is_ok() {"recovered"} else {"failed"},
                "retention":"release after diagnosis/acceptance; rebuildable originals reclaimed within seven days after consumers end"});
            let declaration = if let Some(publication) =
                retention::enroll(root, "check-original", &directory, outcome.clone())?
            {
                let original = publication.protect_consumer(&consumer)?;
                let declaration = serde_json::json!({"schema":"chrono-retention-consumer/v1","consumer":consumer,
                    "purpose":"registered build caller diagnosis and acceptance; release when consumed","originals":[original]});
                publication.complete(outcome)?;
                Some(declaration)
            } else {
                None
            };
            writeln!(stderr, "{}", serde_json::json!({"reason":"chrono-proved-build-original", "path":relative,"consumer":consumer,"report_consumer":declaration})).map_err(|e| e.to_string())?;
            Ok(())
        })();
        match (result, retained) {
            (Err(mut failure), Err(note)) => {
                failure.message += &format!(
                    "; original retention failed: {note}; preserve {}",
                    directory.display()
                );
                Err(failure)
            }
            (Err(mut failure), Ok(())) => {
                failure.message += &format!("; original: {}", directory.display());
                Err(failure)
            }
            (Ok(()), Err(note)) => Err(note.into()),
            (Ok(()), Ok(())) => Ok(()),
        }
    } else {
        temporary.close().map_err(|e| e.to_string())?;
        result
    }
}

#[cfg(not(verus_keep_ghost))]
fn run_attempt(
    root: &Path,
    declaration: &str,
    inherited: &Environment,
    stdout: &mut (impl Write + Send),
    stderr: &mut (impl Write + Send),
    can_recover: bool,
    directory: &Path,
) -> Result<(), BuildFailure> {
    let raw = profile::read(root, declaration, b".chrono-harness/ci/")?;
    let config: Config =
        serde_json::from_value(crate::codec::json(&raw)?).map_err(|e| e.to_string())?;
    if !cargo::registered_sources(
        &config.schema,
        &config.library.source,
        &config.library.sources,
    ) {
        return Err("E_VERUS_BUILD: invalid build registration".into());
    }
    compiler::compiler::<()>(
        json!({"operation":"validate-contract","contract":config.output_contract}),
    )?;
    let manifest = profile::path(root, &config.manifest, b"")?;
    let target = profile::path(root, &config.target, b"")?;
    let verus = profile::path(root, &config.verus_directory, b".chrono-harness/bin/")?;
    let executable = profile::path(root, &config.cargo_verus, b".chrono-harness/bin/")?;
    let projects = profile::read(root, &config.projects, b".chrono-harness/")?;
    let mut methods = execution::methods(&projects)?;
    let mut recovery = methods
        .remove(&config.recovery_operation)
        .ok_or("E_VERUS_BUILD: missing registered cache recovery")?;
    if !crate::verified_bindings::unique_method(recovery.len()) {
        return Err("E_VERUS_BUILD: ambiguous registered cache recovery".into());
    }
    let recovery = recovery.pop().ok_or("missing registered cache recovery")?;
    let rustflags = vec![
        "-Zbinary-dep-depinfo".to_owned(),
        "-Zchecksum-hash-algorithm=sha256".to_owned(),
        "--extern".to_owned(),
        format!("vstd={}", verus.join("libvstd.rlib").display()),
        "-L".to_owned(),
        format!("dependency={}", verus.display()),
    ];
    let method = execution::Method {
        owner: config.owner.clone(),
        operation: config.operation.clone(),
        tool: "cargo-verus".into(),
        argv: vec![
            "build".into(),
            "-v".into(),
            "--fwd-verus-args-to".into(),
            "all".into(),
            "--locked".into(),
            "--manifest-path".into(),
            config.manifest.clone(),
            "--target-dir".into(),
            config.target.clone(),
            "--config".into(),
            format!(
                "build.rustflags={}",
                serde_json::to_string(&rustflags).map_err(|e| e.to_string())?
            ),
            "--message-format=json".into(),
            // Verus 报告没有目标标识，须防止并行编译插入其他库的产物记录。
            // Cargo 专用参数须在 Verus 相关 Cargo 参数之后。
            "--jobs".into(),
            "1".into(),
            "--".into(),
            "--import".into(),
            format!("vstd={}", verus.join("vstd.vir").display()),
            "--no-cheating".into(),
            "--no-external-by-default".into(),
            "--num-threads".into(),
            "2".into(),
            "--output-json".into(),
            "-Zchecksum-hash-algorithm=sha256".into(),
        ],
        output_contract: None,
    };
    let mut environment = inherited.clone();
    environment.insert(
        "RUSTUP_TOOLCHAIN".into(),
        config.rust_toolchain.clone().into(),
    );
    environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
    let mut tools = BTreeMap::new();
    for name in [
        "cargo-verus",
        "verus",
        "rust_verify",
        "z3",
        "libvstd.rlib",
        "vstd.vir",
    ] {
        let file = verus.join(name);
        tools.insert(file.clone(), file_identity(&file)?);
    }
    tools.insert(executable.clone(), file_identity(&executable)?);
    let required = config
        .library
        .sources
        .iter()
        .map(|p| profile::path(root, p, b""))
        .collect::<Result<Vec<_>, _>>()?;
    let mut inputs = BTreeMap::new();
    for source in required.iter().chain([
        &manifest,
        &profile::path(root, &config.library.manifest, b"")?,
        &profile::path(root, declaration, b".chrono-harness/ci/")?,
    ]) {
        inputs.insert(
            source.to_string_lossy().into_owned(),
            file_identity(source)?,
        );
    }
    let key = crate::codec::digest(&(
        crate::files::sha256(&raw),
        &tools,
        &inputs,
        &method.argv,
        &recovery,
    ))?;
    let operation = execution::run_operation(
        root,
        &directory,
        0,
        &method,
        &executable,
        &environment,
        &mut *stdout,
        &mut *stderr,
    )?;
    persist(&directory.join("operation.json"), &operation)?;
    if operation.exit_code != 0 {
        return Err(BuildFailure {
            message: "E_VERUS_BUILD: official compiler process failed".into(),
            exit_code: Some(operation.exit_code),
        });
    }
    let output = bounded(&root.join(&operation.stdout.path))?;
    let errors = bounded(&root.join(&operation.stderr.path))?;
    stream_matches(root, &operation.stdout, &output)?;
    stream_matches(root, &operation.stderr, &errors)?;
    let cache = target.join(".chrono-verus-proof.json");
    let prior = bounded(&cache)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Proof>(&bytes).ok());
    let library_manifest = profile::path(root, &config.library.manifest, b"")?;
    let library_source = profile::path(root, &config.library.source, b"")?;
    let binary_source = profile::path(root, &config.binary.source, b"")?;
    let targets = BuildTargets {
        library: TargetIdentity {
            manifest: library_manifest
                .to_str()
                .ok_or("non UTF-8 library manifest")?
                .into(),
            source: library_source
                .to_str()
                .ok_or("non UTF-8 library source")?
                .into(),
            name: config.library.target.clone(),
        },
        binary: TargetIdentity {
            manifest: manifest.to_str().ok_or("non UTF-8 binary manifest")?.into(),
            source: binary_source
                .to_str()
                .ok_or("non UTF-8 binary source")?
                .into(),
            name: config.binary.target.clone(),
        },
        published: profile::path(root, &config.binary.published, b"")?
            .to_str()
            .ok_or("non UTF-8 binary")?
            .into(),
    };
    let decoded = records(&output, &config.output_contract)?;
    if !cargo::complete_records(&decoded, &targets) {
        return Err("E_VERUS_BUILD: missing or invalid complete original build result".into());
    }
    let mut library = None;
    let mut binary = None;
    let mut reused = false;
    for record in decoded {
        match record {
            Record::Cargo(Cargo::Artifact(artifact)) => {
                if selected(
                    &artifact,
                    &library_manifest,
                    &library_source,
                    &config.library.target,
                    "lib",
                ) {
                    let original: Result<(Vec<Identity>, Vec<u8>), String> = (|| {
                        let (dependency, outputs) = library_paths(root, &config, &artifact)?;
                        let metadata = bind_dependency(
                            &library_manifest,
                            &dependency,
                            &outputs,
                            &required,
                            SourcePaths::Literal,
                        )?;
                        let identities = outputs
                            .iter()
                            .map(|p| identity(p))
                            .collect::<Result<Vec<_>, _>>()?;
                        if !cargo::same_identities(&identities, &identities) {
                            return Err("E_VERUS_BUILD: invalid library identities".into());
                        }
                        if artifact.fresh {
                            let proof = prior.as_ref().ok_or(
                                "E_VERUS_BUILD: fresh library has no retained original proof",
                            )?;
                            validate_prior(
                                root,
                                &config,
                                proof,
                                &key,
                                &metadata,
                                &identities,
                                &method,
                                &executable,
                                &targets,
                            )?;
                        }
                        Ok((identities, metadata))
                    })(
                    );
                    let (identities, metadata) = match original {
                        Ok(observation) => observation,
                        Err(error) => {
                            match cargo::recovery(artifact.fresh, false, can_recover) {
                                Recovery::Continue | Recovery::Reject => return Err(error.into()),
                                Recovery::Rebuild => {}
                            }
                            persist(
                                &directory.join("cache-rejected.json"),
                                &serde_json::json!({"error":error,"operation":operation,"recovery_operation":recovery.operation}),
                            )?;
                            preserve_if_present(&cache, &directory.join("rejected-proof.json"))?;
                            if let Ok((dependency, outputs)) =
                                library_paths(root, &config, &artifact)
                            {
                                preserve_if_present(
                                    &dependency,
                                    &directory.join("rejected-dependency.d"),
                                )?;
                                for output in outputs {
                                    let name =
                                        output.file_name().ok_or("missing library filename")?;
                                    preserve_if_present(&output, &directory.join(name))?;
                                }
                            }
                            let search = environment
                                .get(std::ffi::OsStr::new("PATH"))
                                .and_then(|s| s.to_str());
                            let cargo =
                                crate::files::resolve_program(root, &recovery.tool, search)?;
                            let recovered = execution::run_operation(
                                root,
                                &directory,
                                1,
                                &recovery,
                                &cargo,
                                &environment,
                                &mut *stdout,
                                &mut *stderr,
                            )?;
                            persist(&directory.join("recovery.json"), &recovered)?;
                            if recovered.exit_code != 0 {
                                return Err(BuildFailure {
                                    message: "E_VERUS_BUILD: registered cache recovery failed"
                                        .into(),
                                    exit_code: Some(recovered.exit_code),
                                });
                            }
                            return build_attempt(
                                root,
                                declaration,
                                inherited,
                                stdout,
                                stderr,
                                false,
                            );
                        }
                    };
                    reused = artifact.fresh;
                    library = Some((artifact, identities, metadata));
                } else if selected(
                    &artifact,
                    &manifest,
                    &binary_source,
                    &config.binary.target,
                    "bin",
                ) {
                    let published = profile::path(root, &config.binary.published, b"")?;
                    let expected = published.to_str().ok_or("non UTF-8 binary path")?;
                    if !cargo::binary_shape(&artifact, expected) {
                        return Err(
                            "E_VERUS_BUILD: published binary differs from the registered target"
                                .into(),
                        );
                    }
                    let observed = identity(&published)?;
                    if !cargo::same_identity(&observed, &observed) {
                        return Err("E_VERUS_BUILD: invalid binary identity".into());
                    }
                    binary = Some(observed);
                }
            }
            _ => {}
        }
    }
    let (artifact, libraries, dependency) =
        library.ok_or("E_VERUS_BUILD: missing proved library")?;
    let binary = binary.ok_or("E_VERUS_BUILD: missing final binary")?;
    bind_final_binary(root, &config, &manifest, &binary, &libraries)?;
    for (source, before) in &inputs {
        let after = file_identity(Path::new(source))?;
        if !cargo::content_matches(&after.0, after.1, &before.0, before.1) {
            return Err("E_VERUS_BUILD: source changed during compilation".into());
        }
    }
    for (tool, before) in &tools {
        let after = file_identity(tool)?;
        if !cargo::content_matches(&after.0, after.1, &before.0, before.1) {
            return Err("E_VERUS_BUILD: compiler tool changed during compilation".into());
        }
    }
    if !reused {
        let proof = Proof {
            schema: "chrono-library-proof/v1".into(),
            inputs: key,
            operation: operation.clone(),
            stdout: String::from_utf8(output).map_err(|e| e.to_string())?,
            stderr: errors,
            artifact,
            libraries: libraries.clone(),
            dependency: String::from_utf8(dependency).map_err(|e| e.to_string())?,
        };
        persist(&cache, &proof)?;
    }
    let admission = Admission {
        schema: "chrono-proved-build-admission/v1",
        proof_cache: identity(&cache)?,
        library: libraries,
        binary,
        proof_reused: reused,
    };
    writeln!(
        stdout,
        "{}",
        serde_json::to_string(&admission).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn validate_prior(
    root: &Path,
    config: &Config,
    proof: &Proof,
    key: &str,
    dependency: &[u8],
    libraries: &Vec<Identity>,
    method: &execution::Method,
    executable: &Path,
    targets: &BuildTargets,
) -> Result<(), String> {
    if !cargo::retained_header(
        &proof.schema,
        &proof.inputs,
        key,
        proof.operation.exit_code,
        proof.artifact.fresh,
        proof.dependency.as_bytes(),
        dependency,
    ) || !cargo::same_identities(&proof.libraries, libraries)
    {
        return Err("E_VERUS_BUILD: retained proof inputs or library bytes differ".into());
    }
    let mut argv = vec![
        executable
            .to_str()
            .ok_or("non UTF-8 compiler executable")?
            .to_owned(),
    ];
    argv.extend(method.argv.iter().cloned());
    if !crate::verified_bindings::operation_record_matches(
        &proof.operation.tool,
        &method.tool,
        &proof.operation.argv,
        &argv,
        &proof.operation.cwd,
        root.to_str().ok_or("non UTF-8 host root")?,
        proof.operation.pid,
        proof.operation.exit_code,
    ) || !crate::verified_bindings::artifact_identity::equal_text(
        &proof.operation.executable_sha256,
        &file_identity(executable)?.0,
    ) || !crate::verified_bindings::artifact_identity::equal_text(
        &proof.operation.operation,
        &config.operation,
    ) || proof.operation.reused_from.is_some()
    {
        return Err("E_VERUS_BUILD: retained compiler process identity differs".into());
    }
    stream_matches(root, &proof.operation.stdout, proof.stdout.as_bytes())?;
    stream_matches(root, &proof.operation.stderr, &proof.stderr)?;
    let decoded = records(proof.stdout.as_bytes(), &config.output_contract)?;
    if !cargo::complete_records(&decoded, targets) {
        return Err("E_VERUS_BUILD: retained proof has no complete original result".into());
    }
    for record in &decoded {
        if let Record::Cargo(Cargo::Artifact(artifact)) = record {
            if cargo::target_selected(artifact, &targets.library, "lib")
                && !cargo::same_artifact(artifact, &proof.artifact)
            {
                return Err(
                    "E_VERUS_BUILD: retained library does not match original Cargo output".into(),
                );
            }
        }
    }
    Ok(())
}

#[cfg(not(verus_keep_ghost))]
fn bind_final_binary(
    root: &Path,
    config: &Config,
    manifest: &Path,
    binary: &Identity,
    libraries: &[Identity],
) -> Result<(), String> {
    let directory = profile::path(root, &config.target, b"")?.join("debug/deps");
    let mut found = false;
    let mut required = vec![profile::path(root, &config.binary.source, b"")?];
    required.extend(libraries.iter().map(|file| PathBuf::from(&file.path)));
    let mut count = 0;
    for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !cargo::selected_dependency(name.as_bytes(), config.binary.dependency_prefix.as_bytes())
        {
            continue;
        }
        count += 1;
        if !cargo::output_inventory_within_bound(count) {
            return Err(
                "E_VERUS_BUILD: compiler output inventory exceeds its registered bound".into(),
            );
        }
        let output = path.with_extension("");
        if !output.is_file() {
            continue;
        }
        let observed = identity(&output)?;
        if !cargo::same_content(&observed, binary) {
            continue;
        }
        bind_dependency(manifest, &path, &[output], &required, SourcePaths::Literal)?;
        found = true;
    }
    if !found {
        return Err(
            "E_VERUS_BUILD: final binary has no matching compiler checksum original".into(),
        );
    }
    Ok(())
}

#[cfg(verus_keep_ghost)]
use super::artifact_identity::{equal_bytes, equal_text};
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::artifact_identity::{equal_bytes, equal_text};
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub enum OwnerSourceMode { Native, Proof, Unknown }

/// Empty source declarations fail before opening the requested dep-info.
pub fn owner_source_mode(kind: &str, count: usize) -> (mode: Option<OwnerSourceMode>)
    ensures mode.is_none() == (count == 0),
        match mode {
            Some(OwnerSourceMode::Native) => kind@ == "native"@,
            Some(OwnerSourceMode::Proof) => kind@ == "proof"@,
            Some(OwnerSourceMode::Unknown) => kind@ != "native"@ && kind@ != "proof"@,
            None => true,
        },
{
    if count == 0 { None }
    else if equal_text(kind, "native") { Some(OwnerSourceMode::Native) }
    else if equal_text(kind, "proof") { Some(OwnerSourceMode::Proof) }
    else { Some(OwnerSourceMode::Unknown) }
}

/// Path ordering is observed by std::path. Metadata and canonicalization come from the filesystem.
pub enum NativeHeader { InspectAlias, Accept, Reject }
pub fn native_header(lexical: std::cmp::Ordering, alias: Option<(bool, std::cmp::Ordering)>)
    -> (action: NativeHeader)
    ensures (action is Accept) == (lexical == std::cmp::Ordering::Equal
                || alias == Some((true, std::cmp::Ordering::Equal))),
        (action is InspectAlias) == (lexical != std::cmp::Ordering::Equal && alias.is_none()),
        (action is Reject) == (lexical != std::cmp::Ordering::Equal && alias.is_some()
                && alias != Some((true, std::cmp::Ordering::Equal))),
{
    match (lexical, alias) {
        (std::cmp::Ordering::Equal, _) | (_, Some((true, std::cmp::Ordering::Equal))) => NativeHeader::Accept,
        (_, None) => NativeHeader::InspectAlias,
        _ => NativeHeader::Reject,
    }
}

pub open spec fn compiler_has_path(paths: Seq<Vec<u8>>, name: Seq<u8>) -> bool {
    exists|i: int| 0 <= i < paths.len() && (#[trigger] paths[i])@ == name
}

fn compiler_contains(paths: &[Vec<u8>], name: &[u8]) -> (found: bool)
    ensures found == compiler_has_path(paths@,name@),
{
    let mut i: usize = 0;
    while i < paths.len()
        invariant i <= paths.len(), forall|j:int| 0 <= j < i ==> (#[trigger] paths@[j])@ != name@,
        decreases paths.len() - i,
    {
        if equal_bytes(paths[i].as_slice(),name) { return true; }
        i += 1;
    }
    false
}

pub fn required_compiler_paths(paths: &[Vec<u8>], required: &[String]) -> (found: bool)
    ensures found == (required.len() > 0 && forall|i:int| 0 <= i < required.len()
        ==> compiler_has_path(paths@,vstd::utf8::encode_utf8((#[trigger] required@[i])@))),
{
    if required.len() == 0 { return false; }
    let mut i: usize = 0;
    while i < required.len()
        invariant i <= required.len(), required.len() > 0, forall|j:int| 0 <= j < i
            ==> compiler_has_path(paths@,vstd::utf8::encode_utf8((#[trigger] required@[j])@)),
        decreases required.len() - i,
    {
        if !compiler_contains(paths,required[i].as_str().as_bytes()) { return false; }
        i += 1;
    }
    true
}
}
