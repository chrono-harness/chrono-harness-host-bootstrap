//! Bootstrap configuration rejects duplicate JSON members before decoding typed declarations.
use serde::Deserialize;
use serde_json::Value;
pub fn json(bytes: &[u8]) -> Result<Value, String> {
    let value: serde_yaml_ng::Value =
        serde_json::from_slice(bytes).map_err(|error: serde_json::Error| {
            let message = error.to_string();
            if let Some((quoted, location)) = message
                .strip_prefix("duplicate entry with key ")
                .and_then(|tail| tail.rsplit_once(" at line "))
                && let Ok(key) = litrs::StringLit::parse(quoted)
            {
                return format!("duplicate JSON member {} at line {location}", key.value());
            }
            message
        })?;
    serde_json::to_value(value).map_err(|error| error.to_string())
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|e| e.to_string())
}

pub fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    Ok(crate::files::sha256(
        &serde_jcs::to_vec(value).map_err(|e| e.to_string())?,
    ))
}
#[derive(Deserialize)]
pub struct ProcessResult {
    pub stdout_bytes: Vec<u8>,
    pub stderr_bytes: Vec<u8>,
    pub failure: Option<String>,
    pub exit_code: i32,
}
pub fn expanded_process(value: &Value) -> Result<ProcessResult, String> {
    crate::compiler::query::<Result<_, String>>(
        "judge-evidence-v1",
        &serde_json::json!({"operation":"expand-process","process":value}),
    )?
}
