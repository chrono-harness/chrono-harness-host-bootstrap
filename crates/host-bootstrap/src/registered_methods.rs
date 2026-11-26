//! Bootstrap consumes execution-methods before selecting the host build toolchain.
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
/// The registration protocol drops exactly one trailing LF from decoded diagnostics.
pub fn diagnostic_length(bytes: &[u8]) -> (length: usize)
    ensures length == if bytes@.len() > 0 && bytes@[bytes@.len() - 1] == 10 {
        bytes@.len() - 1
    } else {
        bytes@.len() as int
    },
{
    let length = bytes.len();
    if length > 0 && bytes[length - 1] == 10 {
        length - 1
    } else {
        length
    }
}
}

#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::{CompilerError, compiler_output};
#[cfg(not(verus_keep_ghost))]
use serde::{Deserialize, Serialize};
#[cfg(not(verus_keep_ghost))]
use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Output, Stdio},
    thread,
};

#[cfg(not(verus_keep_ghost))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Method {
    pub owner: String,
    pub operation: String,
    pub tool: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_contract: Option<crate::compiler::OutputContract>,
}

#[cfg(not(verus_keep_ghost))]
/// Read the registration response through the supplied command transport.
pub fn read_methods(
    registry: &[u8],
    invoke: impl FnOnce(&str, &str, &[u8]) -> Result<Output, String>,
) -> Result<BTreeMap<String, Vec<Method>>, String> {
    query_methods(registry, |program, argument, input| {
        invoke(program, argument, input).map(|output| (output, Ok(())))
    })
}

#[cfg(not(verus_keep_ghost))]
fn query_methods(
    registry: &[u8],
    invoke: impl FnOnce(&str, &str, &[u8]) -> Result<(Output, std::io::Result<()>), String>,
) -> Result<BTreeMap<String, Vec<Method>>, String> {
    let (output, written) = invoke("chrono-judge-registration", "execution-methods", registry)?;
    let stdout = compiler_output(
        output.status.success(),
        output.stdout,
        output.stderr,
        written,
    )
    .map_err(|error| match error {
        CompilerError::Status => format!("registration query failed: {}", output.status),
        CompilerError::Write(error) => error.to_string(),
        CompilerError::Diagnostic(bytes) => match String::from_utf8(bytes) {
            Ok(mut reason) => {
                reason.truncate(diagnostic_length(reason.as_bytes()));
                reason
            }
            Err(error) => error.to_string(),
        },
    })?;
    crate::codec::decode(&stdout)
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn methods(registry: &[u8]) -> Result<BTreeMap<String, Vec<Method>>, String> {
    query_methods(registry, |program, argument, input| {
        let executable = crate::files::resolve_program(std::path::Path::new("."), program, None)?;
        let search = std::env::var_os("PATH").ok_or("PATH is missing")?;
        let mut child = Command::new(executable)
            .env_clear()
            .env("PATH", search)
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
            Ok((output, written))
        })
    })
}
