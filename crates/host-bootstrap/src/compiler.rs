//! Calls to the installed runner's public JSON commands.
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::{CompilerError, compiler_output};
#[cfg(not(verus_keep_ghost))]
use serde::{Serialize, de::DeserializeOwned};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs::File,
    io::{BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    thread,
};

#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(not(verus_keep_ghost))]
pub fn query<T: DeserializeOwned>(operation: &str, input: &impl Serialize) -> Result<T, String> {
    let mut data = serde_json::to_vec(input).map_err(|e| e.to_string())?;
    data.push(b'\n');
    let mut child = Command::new("chrono-harness")
        .arg(operation)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("chrono-harness {operation}: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped runner stdin");
    let (output, written) = thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(&data));
        let output = child.wait_with_output();
        (output, writer.join())
    });
    let output = output.map_err(|e| e.to_string())?;
    let written = written
        .map_err(|_| String::from("runner stdin writer panicked"))
        .and_then(|result| result.map_err(|error| error.to_string()));
    let stdout = compiler_output(
        output.status.success(),
        output.stdout,
        output.stderr,
        written,
    )
    .map_err(|error| match error {
        CompilerError::Status => format!("chrono-harness {operation}: {}", output.status),
        CompilerError::Diagnostic(bytes) => match String::from_utf8(bytes) {
            Ok(message) => message.trim_end_matches('\n').to_owned(),
            Err(error) => error.to_string(),
        },
        CompilerError::Write(error) => error,
    })?;
    serde_json::from_slice(&stdout).map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
pub fn compiler<T: DeserializeOwned>(input: Value) -> Result<T, String> {
    query("compiler-output-v1", &input)
}

#[cfg(not(verus_keep_ghost))]
pub fn native_cargo_library(manifest: &str, original: &Path) -> Result<Vec<u8>, String> {
    use crate::verified_cargo::{NativeLibrary, native_dep_info_stem, select_native_library};

    let file = File::open(original).map_err(|error| format!("{}: {error}", original.display()))?;
    let rows = serde_json::Deserializer::from_reader(BufReader::new(file))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", original.display()))?;
    match select_native_library(&rows, manifest) {
        NativeLibrary::InvalidFilenames => {
            Err("matched native Cargo artifact has no filenames".into())
        }
        NativeLibrary::Missing => Err("no native Cargo metadata artifact matched".into()),
        NativeLibrary::Ambiguous => Err("multiple native Cargo metadata artifacts matched".into()),
        NativeLibrary::Unique(path) => Ok(native_dep_info_stem(path.as_bytes())),
    }
}

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputContract {
    pub version: String,
    pub commit: String,
}

#[cfg(not(verus_keep_ghost))]
pub fn proof_rows(contract: &OutputContract, rows: &[&str]) -> Result<Vec<Option<bool>>, String> {
    let result: Vec<Option<bool>> =
        compiler(json!({"operation":"proof-rows","contract":contract,"rows":rows}))?;
    checked_proof_rows(rows.len(), result).map_err(str::to_owned)
}

verus! {
pub fn checked_proof_rows(expected: usize, replies: Vec<Option<bool>>)
    -> (result: Result<Vec<Option<bool>>, &'static str>)
    ensures match result {
        Ok(rows) => replies@.len() == expected && rows@ == replies@,
        Err(message) => replies@.len() != expected
            && message@ == "E_VERUS_BUILD: compiler response row count differs"@,
    },
{
    if replies.len() == expected {
        Ok(replies)
    } else {
        Err("E_VERUS_BUILD: compiler response row count differs")
    }
}
}
