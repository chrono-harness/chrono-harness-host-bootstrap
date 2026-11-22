#[path = "../../host-bootstrap/src/registered_methods.rs"]
mod registered_methods;

use std::{
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
};

#[test]
fn bootstrap_queries_the_supplied_registry_and_keeps_duplicate_methods() {
    let bytes = br#"{"projects":[],"scripts":[]}"#;
    let result = registered_methods::read_methods(bytes, |program, arg, input| {
        assert_eq!((program, arg, input), ("chrono-judge-registration", "execution-methods", bytes.as_slice()));
        let method = serde_json::json!({"owner":"script:a","operation":"build","tool":"rustc","argv":["file with spaces"]});
        Ok(Output { status: ExitStatus::from_raw(0), stdout: serde_json::to_vec(&serde_json::json!({"build":[method,method]})).unwrap(), stderr: vec![] })
    }).unwrap();
    assert_eq!(result["build"].len(), 2);
    assert_eq!(result["build"][0].argv, ["file with spaces"]);
}

#[test]
fn startup_reports_missing_tools_rejected_registries_and_broken_responses() {
    for (response, error) in [
        (Err("executable not found".into()), "executable not found"),
        (
            Ok(Output {
                status: ExitStatus::from_raw(2 << 8),
                stdout: vec![],
                stderr: b"tool missing\n".to_vec(),
            }),
            "tool missing",
        ),
        (
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: b"[]".to_vec(),
                stderr: vec![],
            }),
            "invalid type",
        ),
    ] {
        let failure = registered_methods::read_methods(b"{}", |_, _, _| response).unwrap_err();
        assert!(failure.contains(error), "{failure}");
    }
}
