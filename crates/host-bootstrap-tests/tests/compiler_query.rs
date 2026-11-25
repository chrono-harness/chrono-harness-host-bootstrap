use std::{fs, os::unix::fs::symlink, process::Command};

#[test]
fn completed_process_and_stdin_writer_choose_the_reported_result() {
    let tool = env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool");
    let dir = tempfile::tempdir().unwrap();
    for alias in ["fixture-compiler-query", "chrono-harness"] {
        symlink(tool, dir.path().join(alias)).unwrap();
    }
    let bad_json = serde_json::from_slice::<bool>(b"{")
        .unwrap_err()
        .to_string();
    let bad_utf8 = String::from_utf8(vec![0xff]).unwrap_err().to_string();
    let pipe = std::io::Error::from_raw_os_error(32).to_string();
    let status = "chrono-harness compiler-output-v1: exit status: 17";
    for (case, expected) in [
        ("ok", Ok(true)),
        ("bad-json", Err(bad_json.as_str())),
        ("diagnostic", Err("diagnostic λ")),
        ("invalid-utf8", Err(bad_utf8.as_str())),
        ("newlines", Err("")),
        ("status", Err(status)),
        ("closed-ok", Err(pipe.as_str())),
        ("closed-bad-json", Err(pipe.as_str())),
        ("closed-diagnostic", Err("diagnostic λ")),
        ("closed-status", Err(status)),
        ("closed-invalid-utf8", Err(bad_utf8.as_str())),
    ] {
        let output = Command::new(dir.path().join("fixture-compiler-query"))
            .arg(case)
            .env_clear()
            .env("PATH", dir.path())
            .env("COMPILER_FIXTURE_CASE", case)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{case}: {:?}", output.stderr);
        let result: Result<bool, String> = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            result.as_ref().map_err(String::as_str),
            expected.as_ref().map_err(|e| *e),
            "{case}"
        );
        let completed = dir.path().join("completed");
        assert!(
            fs::read_to_string(&completed)
                .unwrap()
                .parse::<u32>()
                .unwrap()
                > 0
        );
        fs::remove_file(completed).unwrap();
        if !case.starts_with("closed-") {
            let request = dir.path().join("request");
            assert_eq!(
                fs::read(&request).unwrap(),
                "{\"source\":\"λ\"}\n".as_bytes()
            );
            fs::remove_file(request).unwrap();
        }
    }
}
