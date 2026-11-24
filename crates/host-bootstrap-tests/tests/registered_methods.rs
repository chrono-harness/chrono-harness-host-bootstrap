use chrono_host_bootstrap::execution::read_methods;

use std::{
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
};

#[test]
fn bootstrap_queries_the_supplied_registry_and_keeps_duplicate_methods() {
    let bytes = br#"{"projects":[],"scripts":[]}"#;
    let result = read_methods(bytes, |program, arg, input| {
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
        let failure = read_methods(b"{}", |_, _, _| response).unwrap_err();
        assert!(failure.contains(error), "{failure}");
    }
}

#[test]
fn owner_checks_submit_original_compiler_bytes_and_exact_file_identities() {
    use chrono_host_bootstrap::proved_build::check_owner_sources;
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::fs;

    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("proof.rlib");
    let dependency = root.path().join("proof.rlib.d");
    let source = root.path().join("source with spaces.rs");
    let raw = b"original dep-info\n";
    fs::write(&dependency, raw).unwrap();
    fs::write(&source, b"verified source").unwrap();
    let path = source.to_str().unwrap().to_owned();
    let sources = [path.clone()];
    for verdict in [false, true] {
        let result = check_owner_sources("proof", &output, &sources, |request| {
            assert_eq!(
                request,
                json!({"operation":"dependency-matches","dependency":raw.as_slice(),
                "targets":[dependency.as_os_str().as_encoded_bytes(),output.as_os_str().as_encoded_bytes()],
                "sources":[{"path":path.as_bytes(),"length":15,"sha256":format!("{:x}",Sha256::digest(b"verified source")).as_bytes()}]})
            );
            Ok(json!(verdict))
        });
        assert_eq!(result.is_ok(), verdict);
    }
    for observed in [json!([path.as_bytes()]), json!([]), json!(null), json!({})] {
        let result = check_owner_sources("native", &output, &sources, |request| {
            assert_eq!(
                request,
                json!({"operation":"dependency-paths","dependency":raw.as_slice(),"stem":output.as_os_str().as_encoded_bytes()})
            );
            Ok(observed.clone())
        });
        assert_eq!(result.is_ok(), observed == json!([path.as_bytes()]));
    }
    assert!(check_owner_sources("proof", &output, &[], |_| panic!()).is_err());
    fs::remove_file(dependency).unwrap();
    assert!(check_owner_sources("native", &output, &sources, |_| panic!()).is_err());
}
