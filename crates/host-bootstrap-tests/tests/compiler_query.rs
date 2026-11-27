#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use std::{fs, os::unix::fs::symlink, process::Command};

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
#[test]
fn proof_classification_checks_count_and_preserves_row_positions() {
    let tool = env!("CARGO_BIN_EXE_chrono-test-bootstrap-tool");
    let dir = tempfile::tempdir().unwrap();
    for alias in ["fixture-compiler-query", "chrono-harness"] {
        symlink(tool, dir.path().join(alias)).unwrap();
    }
    let mismatch = "E_VERUS_BUILD: compiler response row count differs";
    for case in ["short", "long", "mixed"] {
        let output = Command::new(dir.path().join("fixture-compiler-query"))
            .arg("proof-rows")
            .env_clear()
            .env("PATH", dir.path())
            .env("COMPILER_FIXTURE_CASE", case)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{case}: {:?}", output.stderr);
        let result: Result<Vec<Option<bool>>, String> =
            serde_json::from_slice(&output.stdout).unwrap();
        match case {
            "mixed" => assert_eq!(
                result.unwrap(),
                [None, Some(false), Some(false), Some(true)]
            ),
            _ => assert_eq!(result.unwrap_err(), mismatch),
        }
        let request = dir.path().join("request");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&request).unwrap()).unwrap(),
            serde_json::json!({
                "operation": "proof-rows",
                "contract": {"version": "fixture-version", "commit": "fixture-commit"},
                "rows": ["cargo λ", "proof", "proof", "ordinary"]
            })
        );
        fs::remove_file(request).unwrap();
        fs::remove_file(dir.path().join("completed")).unwrap();
    }
}

#[cfg(verus_keep_ghost)]
use super::{expect, policy::compiler::checked_proof_rows};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {
/// A caller pairs each reply with the output line at the same index.
fn row_positions(expected: usize, replies: Vec<Option<bool>>) {
    let ghost observed = replies@;
    match checked_proof_rows(expected, replies) {
        Ok(rows) => {
            expect(rows.len() == expected);
            proof { assert(rows@ == observed); }
        }
        Err(message) => {
            proof {
                assert(observed.len() != expected);
                assert(message@ == "E_VERUS_BUILD: compiler response row count differs"@);
            }
        }
    }
}
}
