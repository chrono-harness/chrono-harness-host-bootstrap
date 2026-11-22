//! Bootstrap consumes execution-methods before selecting the host build toolchain.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Output, Stdio},
    thread,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<chrono_harness::verified_verifier::OutputContract>,
}

pub(crate) fn read_methods(
    registry: &[u8],
    invoke: impl FnOnce(&str, &str, &[u8]) -> Result<Output, String>,
) -> Result<BTreeMap<String, Vec<Method>>, String> {
    let output = invoke("chrono-judge-registration", "execution-methods", registry)?;
    if !output.status.success() {
        let reason = String::from_utf8(output.stderr).map_err(|error| error.to_string())?;
        return Err(if reason.is_empty() {
            format!("registration query failed: {}", output.status)
        } else {
            reason.strip_suffix('\n').unwrap_or(&reason).to_owned()
        });
    }
    chrono_harness::decode(&output.stdout)
}

pub(crate) fn methods(registry: &[u8]) -> Result<BTreeMap<String, Vec<Method>>, String> {
    read_methods(registry, |program, argument, input| {
        let executable = chrono_harness::resolve_program(std::path::Path::new("."), program, None)?;
        let mut child = Command::new(executable)
            .env_clear()
            .arg(argument)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{program}: {error}"))?;
        let mut stdin = child.stdin.take().expect("piped child stdin");
        thread::scope(|scope| {
            let writer = scope.spawn(move || stdin.write_all(input));
            let result = child.wait_with_output();
            let written = writer
                .join()
                .map_err(|_| "registration input writer panicked")?;
            let output = result.map_err(|error| error.to_string())?;
            if output.status.success() {
                written.map_err(|error| error.to_string())?;
            }
            Ok(output)
        })
    })
}
