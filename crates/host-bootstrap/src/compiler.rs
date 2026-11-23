//! Calls to the installed runner's public JSON commands.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
};

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
    if !output.status.success() {
        let message = String::from_utf8(output.stderr).map_err(|e| e.to_string())?;
        return Err(if message.is_empty() {
            format!("chrono-harness {operation}: {}", output.status)
        } else {
            message.trim_end_matches('\n').to_owned()
        });
    }
    written
        .map_err(|_| "runner stdin writer panicked")?
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}

pub fn compiler<T: DeserializeOwned>(input: Value) -> Result<T, String> {
    query("compiler-output-v1", &input)
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputContract {
    pub version: String,
    pub commit: String,
}

pub fn proof_rows(contract: &OutputContract, rows: &[&str]) -> Result<Vec<Option<bool>>, String> {
    let result: Vec<Option<bool>> =
        compiler(json!({"operation":"proof-rows","contract":contract,"rows":rows}))?;
    if result.len() != rows.len() {
        return Err("E_VERUS_BUILD: compiler response row count differs".into());
    }
    Ok(result)
}
