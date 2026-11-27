//! Bind independently collected Cargo-Verus library results to one serial build.
#[cfg(verus_keep_ghost)]
use super::{ascii_component, valid_ascii_component};
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::valid_ascii_component;
use vstd::prelude::*;

verus! {
/// Cargo creates a proof operation under the caller's canonical host directory.
pub fn operation_directory(inside_host: bool, operation: &[u8]) -> (accepted: bool)
    ensures accepted == (inside_host && ascii_component(operation@)),
{
    inside_host && valid_ascii_component(operation)
}
}

#[cfg(not(verus_keep_ghost))]
use crate::bindings::Environment;
#[cfg(not(verus_keep_ghost))]
use crate::compiler::{OutputContract, proof_rows};
#[cfg(not(verus_keep_ghost))]
use crate::execution::{self, Method, Operation};
#[cfg(not(verus_keep_ghost))]
use crate::files::{checked_path, file_identity, resolve_program};
#[cfg(not(verus_keep_ghost))]
use crate::original::operation_record_matches;
#[cfg(not(verus_keep_ghost))]
use crate::proved_build::{SourcePaths, bind_dependency, bounded, persist};
#[cfg(not(verus_keep_ghost))]
use crate::verified_cargo::{self as cargo, Artifact, Cargo, SerialEvent, SerialProofs};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use serde_json::value::RawValue;
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[cfg(not(verus_keep_ghost))]
type Identity = (String, u64);

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    targets: Vec<crate::verified_cargo::Target>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Library {
    package: String,
    package_id: String,
    kind: String,
    target: String,
    manifest: PathBuf,
    source: PathBuf,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    packages: Vec<String>,
    process: Operation,
    inventory: BTreeMap<String, Library>,
    files: BTreeMap<PathBuf, Identity>,
}

#[cfg(not(verus_keep_ghost))]
struct Original<'a> {
    artifact: Artifact,
    proof: &'a str,
}

#[cfg(not(verus_keep_ghost))]
fn io_error(error: io::Error) -> String {
    error.to_string()
}

/// Cargo supplies the package ID and target. With --jobs 1 the verifier's raw
/// object precedes its own library artifact. No work is launched by the reader.
#[cfg(not(verus_keep_ghost))]
fn collect_records<'a>(
    mut bytes: &'a [u8],
    libraries: &BTreeMap<String, Library>,
) -> Result<BTreeMap<String, Original<'a>>, String> {
    let mut pending = None;
    let mut state = SerialProofs::new(libraries.len());
    let mut results = BTreeMap::new();
    while !bytes.trim_ascii_start().is_empty() {
        bytes = bytes.trim_ascii_start();
        // cargo-verus prints the standard library's human summary even when
        // Cargo's message format is JSON and only workspace roots use JSON.
        if let Some(remainder) = bytes.strip_prefix(b"verification results:: ") {
            let (line, remaining) = remainder.split_at(
                remainder
                    .iter()
                    .position(|b| *b == b'\n')
                    .ok_or("truncated dependency summary")?,
            );
            let count = line
                .strip_suffix(b" verified, 0 errors")
                .ok_or("invalid dependency summary")?;
            if !state.accept(SerialEvent::Note)
                || count.is_empty()
                || !count.iter().all(u8::is_ascii_digit)
            {
                return Err("unexpected dependency summary".into());
            }
            bytes = remaining;
            continue;
        }
        let mut stream = serde_json::Deserializer::from_slice(bytes).into_iter::<&RawValue>();
        let raw = stream
            .next()
            .ok_or("missing Cargo record")?
            .map_err(|e| e.to_string())?
            .get();
        bytes = &bytes[stream.byte_offset()..];
        #[derive(Deserialize)]
        struct Header {
            reason: Option<String>,
            #[serde(rename = "verification-results")]
            verification: Option<serde::de::IgnoredAny>,
        }
        let header: Header = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        if header.reason.is_none() && header.verification.is_some() {
            if !state.accept(SerialEvent::Proof) {
                return Err("unbound verifier reports".into());
            }
            pending = Some(raw);
            continue;
        }
        let record: Cargo = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        match record {
            Cargo::Artifact(artifact) => {
                let requested = libraries.iter().enumerate().find(|(_, (_, lib))| {
                    cargo::library_selected(
                        &artifact,
                        &lib.package_id,
                        &lib.manifest.to_string_lossy(),
                        &lib.source.to_string_lossy(),
                        &lib.target,
                        &lib.kind,
                    )
                });
                let selected = requested.as_ref().map(|(i, _)| *i);
                if !state.accept(SerialEvent::Artifact {
                    selected,
                    fresh: artifact.fresh,
                }) {
                    return Err(format!(
                        "unbound Cargo result for {} in {}",
                        artifact.target.name, artifact.package_id
                    ));
                }
                if let Some((_, (name, _))) = requested {
                    let proof = pending
                        .take()
                        .ok_or_else(|| format!("missing verifier report for {name}"))?;
                    results.insert(name.clone(), Original { artifact, proof });
                }
            }
            other => {
                let event = match other {
                    Cargo::Finished { success } => SerialEvent::Finished { success },
                    Cargo::Message { message } if !cargo::diagnostic_rejected(&message.level) => {
                        SerialEvent::Note
                    }
                    Cargo::Script { .. } => SerialEvent::Note,
                    _ => SerialEvent::Error,
                };
                if !state.accept(event) {
                    return Err("invalid Cargo evidence sequence".into());
                }
            }
        }
    }
    if !state.complete() {
        let missing: Vec<_> = libraries
            .keys()
            .filter(|name| !results.contains_key(*name))
            .collect();
        return Err(format!("incomplete Cargo proof inventory: {missing:?}"));
    }
    Ok(results)
}

#[cfg(not(verus_keep_ghost))]
pub struct Build {
    root: PathBuf,
    directory: PathBuf,
    manifest: String,
    tool: PathBuf,
    operation: String,
}
#[cfg(not(verus_keep_ghost))]
impl Build {
    pub fn new(
        root: &Path,
        directory: &str,
        operation: &str,
        tool: &str,
        manifest: &str,
    ) -> Result<Self, String> {
        let root = root.canonicalize().map_err(io_error)?;
        let directory = root.join(directory).canonicalize().map_err(io_error)?;
        if !operation_directory(directory.starts_with(&root), operation.as_bytes()) {
            return Err("invalid Cargo proof operation directory".into());
        }
        checked_path(&root, manifest)?;
        let tool = resolve_program(&root, tool, None)?
            .canonicalize()
            .map_err(io_error)?;
        Ok(Self {
            root,
            directory: directory.join(operation),
            manifest: manifest.into(),
            tool,
            operation: operation.into(),
        })
    }

    fn method(&self, packages: &[String]) -> Result<Method, String> {
        if packages.is_empty()
            || packages.iter().collect::<BTreeSet<_>>().len() != packages.len()
            || packages.iter().any(|p| {
                p.is_empty()
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            })
        {
            return Err("expected unique Cargo package names".into());
        }
        let mut argv = vec![
            "build".into(),
            "--locked".into(),
            "--fwd-verus-args-to".into(),
            "roots".into(),
            "--manifest-path".into(),
            self.manifest.clone(),
        ];
        for p in packages {
            argv.extend(["--package".into(), p.clone()]);
        }
        argv.extend([
            "--target-dir".into(),
            self.directory
                .join("target")
                .to_str()
                .ok_or("target path must be UTF-8")?
                .into(),
            "--config".into(),
            r#"build.rustflags=["-Zbinary-dep-depinfo","-Zchecksum-hash-algorithm=sha256"]"#.into(),
            "--lib".into(),
            "--examples".into(),
            "--message-format=json".into(),
            "--jobs".into(),
            "1".into(),
            "--".into(),
            "--no-cheating".into(),
            "--no-external-by-default".into(),
            "--num-threads".into(),
            "2".into(),
            "--output-json".into(),
            "-Zchecksum-hash-algorithm=sha256".into(),
        ]);
        Ok(Method {
            owner: "host".into(),
            operation: self.operation.clone(),
            tool: "cargo-verus".into(),
            argv,
            output_contract: None,
        })
    }

    fn output_paths(&self, artifact: &Artifact) -> Result<(PathBuf, Vec<PathBuf>), String> {
        let debug = self.directory.join("target/debug");
        let [reported, metadata] = artifact.filenames.as_slice() else {
            return Err("unexpected Cargo library outputs".into());
        };
        let target = &artifact.target.name;
        let (fingerprints, published_dir) = match artifact.target.kind.as_slice() {
            [kind] if kind == "lib" => (debug.join("deps"), debug.clone()),
            [kind] if kind == "example" => (debug.join("examples"), debug.join("examples")),
            _ => return Err(format!("unsupported Cargo target {target}")),
        };
        let metadata = PathBuf::from(metadata);
        let library = metadata.with_extension("rlib");
        let published = published_dir.join(format!("lib{target}.rlib"));
        let reported = Path::new(reported);
        if reported != library && reported != published {
            return Err(format!("unexpected published library for {target}"));
        }
        let names = vec![
            library.to_str().ok_or("invalid library path")?.to_owned(),
            artifact.filenames[1].clone(),
        ];
        let layout = crate::verified_cargo::compiler_library_paths(
            fingerprints.as_os_str().as_encoded_bytes(),
            target.as_bytes(),
            &names,
            artifact.executable.is_some(),
        )
        .ok_or_else(|| format!("invalid Cargo fingerprint paths for {target}"))?;
        let dependency =
            PathBuf::from(String::from_utf8(layout.dependency).map_err(|e| e.to_string())?);
        let mut files = vec![library, metadata.clone(), metadata.with_extension("vir")];
        if reported == published {
            files.push(published);
        }
        Ok((dependency, files))
    }

    fn outputs(&self, artifact: &Artifact) -> Result<(PathBuf, Vec<PathBuf>), String> {
        let (dependency, files) = self.output_paths(artifact)?;
        if let [library, _, _, published] = files.as_slice() {
            if file_identity(published)? != file_identity(library)? {
                return Err(format!(
                    "published library differs for {}",
                    artifact.target.name
                ));
            }
        }
        Ok((dependency, files))
    }

    fn inputs(
        &self,
        libraries: &BTreeMap<String, Library>,
    ) -> Result<BTreeMap<PathBuf, Identity>, String> {
        let mut paths = vec![
            self.root.join(&self.manifest),
            self.root.join(&self.manifest).with_file_name("Cargo.lock"),
            self.tool.clone(),
        ];
        let dist = self.tool.parent().ok_or("invalid cargo-verus path")?;
        for name in ["verus", "rust_verify", "z3", "vstd.vir", "libvstd.rlib"] {
            paths.push(dist.join(name));
        }
        for lib in libraries.values() {
            paths.push(lib.manifest.clone());
            paths.push(lib.source.clone());
        }
        Self::identities(paths)
    }

    fn files(
        &self,
        originals: &BTreeMap<String, Original<'_>>,
    ) -> Result<BTreeMap<PathBuf, Identity>, String> {
        let mut paths = Vec::new();
        for original in originals.values() {
            let (dep, files) = self.outputs(&original.artifact)?;
            paths.push(dep);
            paths.extend(files);
        }
        Self::identities(paths)
    }

    fn identities(paths: Vec<PathBuf>) -> Result<BTreeMap<PathBuf, Identity>, String> {
        paths
            .into_iter()
            .map(|p| {
                let identity = file_identity(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                if identity.1 == 0 {
                    return Err(format!("empty compiler artifact {}", p.display()));
                }
                Ok((p, identity))
            })
            .collect()
    }

    pub fn run(&self, packages: &[String], environment: &Environment) -> Result<(), String> {
        let method = self.method(packages)?;
        fs::create_dir(&self.directory).map_err(io_error)?;
        let cargo = resolve_program(&self.root, "cargo", None)?;
        let metadata = crate::bindings::capture(
            &self.root,
            &cargo,
            &[
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--locked",
                "--manifest-path",
                &self.manifest,
            ],
            environment,
        )?;
        let metadata: Metadata =
            serde_json::from_str(&crate::bindings::output_text(metadata, "cargo metadata")?)
                .map_err(|e| e.to_string())?;
        let mut inventory = BTreeMap::new();
        for name in packages {
            let mut candidates = metadata.packages.iter().filter(|p| &p.name == name);
            let package = candidates
                .next()
                .ok_or_else(|| format!("unknown workspace package {name}"))?;
            if candidates.next().is_some() {
                return Err(format!("ambiguous Cargo package {name}"));
            }
            let count = inventory.len();
            for target in &package.targets {
                let [kind] = target.kind.as_slice() else {
                    continue;
                };
                if !matches!(kind.as_str(), "lib" | "example") {
                    continue;
                }
                if target.crate_types != ["lib"] {
                    return Err(format!(
                        "expected library crate-type for {name}/{}",
                        target.name
                    ));
                }
                let key = format!("{name}/{kind}/{}", target.name);
                let lib = Library {
                    package: name.clone(),
                    package_id: package.id.clone(),
                    kind: kind.clone(),
                    target: target.name.clone(),
                    manifest: package.manifest_path.clone(),
                    source: PathBuf::from(&target.src_path),
                };
                if inventory.insert(key, lib).is_some() {
                    return Err(format!("duplicate Cargo target in {name}"));
                }
            }
            if inventory.len() == count {
                return Err(format!("missing Cargo proof targets for {name}"));
            }
        }
        let before = self.inputs(&inventory)?;
        let mut environment = environment.clone();
        environment.insert("RUSTC_BOOTSTRAP".into(), "1".into());
        let process = execution::run_operation(
            &self.root,
            &self.directory,
            0,
            &method,
            &self.tool,
            &environment,
            io::sink(),
            io::stderr(),
        )?;
        if process.exit_code != 0 {
            return Err(format!(
                "cargo-verus exited {}, original {}",
                process.exit_code, process.stdout.path
            ));
        }
        let raw = bounded(&self.root.join(&process.stdout.path))?;
        let originals = collect_records(&raw, &inventory)?;
        let mut files = self.inputs(&inventory)?;
        if files != before {
            return Err("Cargo proof inputs changed during compilation".into());
        }
        files.extend(self.files(&originals)?);
        persist(
            &self.directory.join("receipt.json"),
            &Receipt {
                packages: packages.to_vec(),
                process,
                inventory,
                files,
            },
        )
    }

    pub fn collect(
        &self,
        package: &str,
        contract: &OutputContract,
        manifest: &str,
        sources: &[String],
    ) -> Result<String, String> {
        self.collect_output(package, contract, manifest, sources, false)
    }

    pub fn collect_original(
        &self,
        package: &str,
        contract: &OutputContract,
        manifest: &str,
        sources: &[String],
    ) -> Result<String, String> {
        self.collect_output(package, contract, manifest, sources, true)
    }

    fn collect_output(
        &self,
        package: &str,
        contract: &OutputContract,
        manifest: &str,
        sources: &[String],
        include_original: bool,
    ) -> Result<String, String> {
        let receipt_path = self.directory.join("receipt.json");
        let receipt_bytes = bounded(&receipt_path)?;
        let receipt: Receipt = serde_json::from_slice(&receipt_bytes).map_err(|e| e.to_string())?;
        let process = &receipt.process;
        let method = self.method(&receipt.packages)?;
        let mut argv = vec![
            self.tool
                .to_str()
                .ok_or("cargo-verus path must be UTF-8")?
                .into(),
        ];
        argv.extend(method.argv);
        let stdout = self.directory.join("0.stdout");
        let stderr = self.directory.join("0.stderr");
        if !operation_record_matches(
            &process.tool,
            "cargo-verus",
            &process.argv,
            &argv,
            &process.cwd,
            self.root.to_str().ok_or("host path must be UTF-8")?,
            process.pid,
            process.exit_code,
        ) || !crate::owner_proof::original_for(
            &process.operation,
            &self.operation,
            process.reused_from.is_some(),
        ) || self.root.join(&process.stdout.path) != stdout
            || self.root.join(&process.stderr.path) != stderr
            || file_identity(&stderr)? != (process.stderr.sha256.clone(), process.stderr.size)
            || file_identity(&self.tool)?.0 != process.executable_sha256
        {
            return Err("Cargo proof process identity differs".into());
        }
        let raw = bounded(&stdout)?;
        if (crate::files::sha256(&raw), raw.len() as u64)
            != (process.stdout.sha256.clone(), process.stdout.size)
        {
            return Err("original Cargo stdout differs".into());
        }
        let originals = collect_records(&raw, &receipt.inventory)?;
        let root_source = checked_path(&self.root, sources.first().ok_or("missing proof root")?)?
            .canonicalize()
            .map_err(io_error)?;
        let manifest = checked_path(&self.root, manifest)?;
        let mut matches = receipt
            .inventory
            .iter()
            .filter(|(_, lib)| lib.package == package && lib.manifest == manifest);
        let mut found = None;
        for (name, lib) in &mut matches {
            if lib.source.canonicalize().map_err(io_error)? == root_source {
                if found.replace(name).is_some() {
                    return Err(format!("ambiguous Cargo proof for {package}"));
                }
            }
        }
        let name = found.ok_or_else(|| format!("missing Cargo proof target for {package}"))?;
        let original = &originals[name];
        let mut files = self.inputs(&receipt.inventory)?;
        files.extend(self.files(&originals)?);
        if let Some(path) = files
            .keys()
            .chain(receipt.files.keys())
            .find(|p| files.get(*p) != receipt.files.get(*p))
        {
            return Err(format!(
                "Cargo proof artifact differs in {} while collecting {package}: {}",
                self.operation,
                path.display()
            ));
        }
        if !cargo::one_successful_proof(&proof_rows(contract, &[original.proof])?) {
            return Err(format!("unverified Cargo proof {package}"));
        }
        let (dep, outputs) = self.outputs(&original.artifact)?;
        let required = sources
            .iter()
            .map(|s| checked_path(&self.root, s))
            .collect::<Result<Vec<_>, _>>()?;
        let dependency = bind_dependency(
            &self.root.join(&self.manifest),
            &dep,
            &outputs[..2],
            &required,
            SourcePaths::Resolved,
        )?;
        if !include_original {
            return Ok(original.proof.into());
        }
        // Collection has already checked the raw serial Cargo stream, target, files,
        // and compiler checksums. Export those same observations before scratch expires.
        let library = &receipt.inventory[name];
        let artifact = &outputs[0];
        let (artifact_sha256, artifact_length) = &receipt.files[artifact];
        serde_json::to_string(&serde_json::json!({
            "schema": "chrono-cargo-verus-root/v1",
            "workspace": self.manifest,
            "receipt": {"path":receipt_path,"sha256":crate::files::sha256(&receipt_bytes),"length":receipt_bytes.len()},
            "target": library,
            "artifact": {"path": artifact, "sha256": artifact_sha256, "length": artifact_length},
            "dep_info": {"path": dep, "original": String::from_utf8(dependency).map_err(|e| e.to_string())?},
            "report_sha256": crate::files::sha256(original.proof.as_bytes()),
        }))
        .map_err(|error| error.to_string())
    }
}
/// Read a root from the same completed serial stream and verify the archived compiler bytes.
/// Archive paths belong to the original host. Replay never opens those absolute paths.
#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    receipt: Receipt,
    stdout: String,
    contract: OutputContract,
    roots: Vec<SelectedRoot>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectedRoot {
    path: String,
    original: RootOriginal,
    sources: BTreeMap<PathBuf, Identity>,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RootOriginal {
    schema: String,
    workspace: String,
    target: Library,
    dep_info: DependencyOriginal,
    artifact: ArtifactIdentity,
    report_sha256: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencyOriginal {
    path: PathBuf,
    original: String,
}

#[cfg(not(verus_keep_ghost))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactIdentity {
    path: PathBuf,
    sha256: String,
    length: u64,
}

#[cfg(not(verus_keep_ghost))]
fn lexical(path: &Path) -> Result<PathBuf, String> {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return Err("compiler source path escapes its root".into());
                }
            }
            std::path::Component::CurDir => {}
            component => result.push(component),
        }
    }
    Ok(result)
}

#[cfg(not(verus_keep_ghost))]
impl Replay {
    pub fn validate(
        &self,
        check: impl FnOnce(&OutputContract, &[&str]) -> Result<Vec<Option<bool>>, String>,
    ) -> Result<BTreeMap<String, String>, String> {
        let process = &self.receipt.process;
        let cwd = Path::new(&process.cwd);
        if !cwd.is_absolute()
            || self.roots.is_empty()
            || lexical(cwd)? != cwd
            || self.stdout.len() as u64 != process.stdout.size
            || crate::files::sha256(self.stdout.as_bytes()) != process.stdout.sha256
        {
            return Err("Cargo archive original stream differs".into());
        }
        let workspace = &self.roots[0].original.workspace;
        let stdout = cwd.join(&process.stdout.path);
        let directory = stdout.parent().ok_or("missing Cargo output parent")?;
        let tool = process.argv.first().ok_or("missing Cargo executable")?;
        let build = Build {
            root: cwd.into(),
            directory: directory.into(),
            manifest: workspace.clone(),
            tool: tool.into(),
            operation: process.operation.clone(),
        };
        let mut argv = vec![tool.clone()];
        argv.extend(build.method(&self.receipt.packages)?.argv);
        if !operation_record_matches(
            &process.tool,
            "cargo-verus",
            &process.argv,
            &argv,
            &process.cwd,
            &process.cwd,
            process.pid,
            process.exit_code,
        ) || process.reused_from.is_some()
            || stdout != directory.join("0.stdout")
            || self.receipt.files.get(Path::new(tool)).map(|id| &id.0)
                != Some(&process.executable_sha256)
        {
            return Err("Cargo archive invocation differs".into());
        }
        let originals = collect_records(self.stdout.as_bytes(), &self.receipt.inventory)?;
        let mut seen = BTreeSet::new();
        let mut reports = Vec::new();
        for selected in &self.roots {
            let row = &selected.original;
            if !seen.insert(&selected.path)
                || row.schema != "chrono-cargo-verus-root/v1"
                || &row.workspace != workspace
                || selected.sources.is_empty()
                || !selected.sources.contains_key(Path::new(&selected.path))
                || Path::new(&selected.path).is_absolute()
                || lexical(Path::new(&selected.path))? != Path::new(&selected.path)
                || lexical(&row.target.source)? != cwd.join(&selected.path)
            {
                return Err("Cargo archive root declaration differs".into());
            }
            let mut matches = self
                .receipt
                .inventory
                .iter()
                .filter(|(_, target)| *target == &row.target);
            let (name, _) = matches.next().ok_or("Cargo archive target absent")?;
            if matches.next().is_some() {
                return Err("Cargo archive target is ambiguous".into());
            }
            let original = &originals[name];
            if crate::files::sha256(original.proof.as_bytes()) != row.report_sha256 {
                return Err("Cargo report differs from its serial target".into());
            }
            reports.push(original.proof);
            let (dep, files) = build.output_paths(&original.artifact)?;
            if dep != row.dep_info.path
                || files[0] != row.artifact.path
                || self.receipt.files.get(&dep)
                    != Some(&(
                        crate::files::sha256(row.dep_info.original.as_bytes()),
                        row.dep_info.original.len() as u64,
                    ))
                || self.receipt.files.get(&files[0])
                    != Some(&(row.artifact.sha256.clone(), row.artifact.length))
                || row.artifact.length == 0
            {
                return Err("Cargo archive compiler artifact or dependency differs".into());
            }
            let mut used = BTreeSet::new();
            let base = cwd.join(workspace);
            let base = base.parent().ok_or("workspace manifest parent missing")?;
            for line in row
                .dep_info
                .original
                .lines()
                .filter_map(|l| l.strip_prefix("# checksum:sha256="))
            {
                let (digest, tail) = line
                    .split_once(" file_len:")
                    .ok_or("malformed Cargo checksum")?;
                let (length, path) = tail.split_once(' ').ok_or("malformed Cargo source path")?;
                let physical = lexical(&base.join(path))?;
                let Ok(relative) = physical.strip_prefix(cwd) else {
                    continue;
                };
                if let Some((wanted, bytes)) = selected.sources.get(relative) {
                    if !used.insert(relative.to_owned())
                        || wanted != digest
                        || length != bytes.to_string()
                    {
                        return Err(format!(
                            "Cargo candidate source differs: {}",
                            relative.display()
                        ));
                    }
                }
            }
            if used.len() != selected.sources.len() {
                return Err("Cargo candidate source missing from compiled target".into());
            }
        }
        let verified = check(&self.contract, &reports)?;
        if verified.len() != reports.len() || verified.iter().any(|v| v != &Some(true)) {
            return Err("Cargo archive lacks an entire-root proof".into());
        }
        Ok(self
            .roots
            .iter()
            .zip(reports)
            .map(|(selected, report)| {
                (
                    selected.path.clone(),
                    crate::files::sha256(report.as_bytes()),
                )
            })
            .collect())
    }
}
