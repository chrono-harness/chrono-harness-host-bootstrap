//! Execution and collection of one host-declared Verus proof in a fresh check directory.
use crate::bindings::Environment;
use crate::compiler::OutputContract;
use crate::execution::{self, Method, Operation};
use crate::files::{file_identity, relative_path, resolve_program};
use crate::original::operation_record_matches;
use crate::proved_build::check_owner_proof;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    process: Operation,
    library: (String, u64),
    dependencies: (String, u64),
}

pub struct Proof {
    root: PathBuf,
    directory: PathBuf,
    verifier: PathBuf,
    method: Method,
}

impl Proof {
    pub fn new(
        root: &Path,
        scratch: &Path,
        operation: &str,
        verifier: &str,
        source: &str,
    ) -> Result<Self, String> {
        if operation.is_empty()
            || operation == "."
            || operation == ".."
            || !operation
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || b"._-".contains(&v))
        {
            return Err("invalid proof operation name".into());
        }
        relative_path(source)?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let scratch = root
            .join(scratch)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !scratch.starts_with(&root) {
            return Err("proof scratch is outside the host".into());
        }
        let directory = scratch.join(operation);
        let verifier = resolve_program(&root, verifier, None)?;
        let library = directory.join("proof.rlib");
        let output = library.to_str().ok_or("non UTF-8 proof output")?;
        let method = Method {
            owner: "host".into(),
            operation: operation.into(),
            tool: "verus".into(),
            argv: vec![
                "--crate-type=lib".into(),
                "--no-cheating".into(),
                "--no-external-by-default".into(),
                "--num-threads".into(),
                "2".into(),
                "--output-json".into(),
                "--compile".into(),
                format!("--emit=link={output},dep-info={output}.d"),
                source.into(),
                "-o".into(),
                output.into(),
                "-Zchecksum-hash-algorithm=sha256".into(),
            ],
            output_contract: None,
        };
        Ok(Self {
            root,
            directory,
            verifier,
            method,
        })
    }

    pub fn run(&self, environment: &Environment) -> Result<(), String> {
        fs::create_dir(&self.directory).map_err(|e| format!("fresh proof operation: {e}"))?;
        let process = execution::run_operation(
            &self.root,
            &self.directory,
            0,
            &self.method,
            &self.verifier,
            environment,
            io::sink(),
            io::stderr(),
        )?;
        if process.exit_code != 0 {
            io::copy(
                &mut fs::File::open(self.root.join(&process.stdout.path))
                    .map_err(|e| e.to_string())?,
                &mut io::stderr(),
            )
            .map_err(|e| e.to_string())?;
            return Err(format!(
                "{} exited {}",
                self.method.operation, process.exit_code
            ));
        }
        let receipt = Receipt {
            process,
            library: file_identity(&self.directory.join("proof.rlib"))?,
            dependencies: file_identity(&self.directory.join("proof.rlib.d"))?,
        };
        let mut file =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|e| e.to_string())?;
        serde_json::to_writer(&mut file, &receipt).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        file.persist_noclobber(self.directory.join("receipt.json"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn collect(
        &self,
        contract: &OutputContract,
        sources: &[String],
        query: impl Fn(Value) -> Result<Value, String>,
    ) -> Result<String, String> {
        let receipt: Receipt = crate::codec::decode(
            &fs::read(self.directory.join("receipt.json")).map_err(|e| e.to_string())?,
        )?;
        let process = receipt.process;
        let stdout = self.directory.join("0.stdout");
        let stderr = self.directory.join("0.stderr");
        let output = self.directory.join("proof.rlib");
        let expected = execution::operation_argv(
            self.verifier.to_str().ok_or("non UTF-8 verifier")?.into(),
            &self.method,
            stdout.strip_prefix(&self.root).map_err(|e| e.to_string())?,
        )?;
        if process.operation != self.method.operation
            || process.reused_from.is_some()
            || !operation_record_matches(
                &process.tool,
                &self.method.tool,
                &process.argv,
                &expected,
                &process.cwd,
                self.root.to_str().ok_or("non UTF-8 root")?,
                process.pid,
                process.exit_code,
            )
            || process.executable_sha256 != file_identity(&self.verifier)?.0
            || self.root.join(&process.stdout.path) != stdout
            || self.root.join(&process.stderr.path) != stderr
            || receipt.library != file_identity(&output)?
            || receipt.dependencies != file_identity(&self.directory.join("proof.rlib.d"))?
            || file_identity(&stderr)? != (process.stderr.sha256, process.stderr.size)
        {
            return Err(format!(
                "{} proof execution identity differs",
                self.method.operation
            ));
        }
        let original = check_owner_proof(contract, &stdout, &output, sources, query)?;
        if (
            crate::files::sha256(original.as_bytes()),
            original.len() as u64,
        ) != (process.stdout.sha256, process.stdout.size)
        {
            return Err(format!(
                "{} verifier stdout differs from process receipt",
                self.method.operation
            ));
        }
        Ok(original)
    }
}
