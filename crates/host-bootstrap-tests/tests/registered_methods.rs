#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::{cargo, expect};
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::verified_cargo as cargo;
use vstd::prelude::*;
#[cfg(not(verus_keep_ghost))]
#[path = "support/expect.rs"]
mod expectation;
#[cfg(not(verus_keep_ghost))]
use expectation::expect;

#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::execution::read_methods;

#[cfg(not(verus_keep_ghost))]
use std::{
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
};

#[test]
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
                "sources":[{"path":path.as_bytes(),"length":15,"sha256":hex::encode(Sha256::digest(b"verified source")).as_bytes()}]})
            );
            Ok(json!(verdict))
        });
        assert_eq!(result.is_ok(), verdict);
    }
    let stem = output.as_os_str().as_encoded_bytes();
    for (observed, expected) in [
        (json!([stem, [path.as_bytes()]]), true),
        (json!([stem, []]), false),
        (json!(null), false),
        (json!({}), false),
    ] {
        let result = check_owner_sources("native", &output, &sources, |request| {
            assert_eq!(
                request,
                json!({"operation":"dependency-header","dependency":raw.as_slice()})
            );
            Ok(observed.clone())
        });
        assert_eq!(result.is_ok(), expected);
    }
    let nested = root.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let alias = nested.join("../proof.rlib");
    assert!(
        check_owner_sources("native", &output, &sources, |_| Ok(json!([
            alias.as_os_str().as_encoded_bytes(),
            [path.as_bytes()]
        ])))
        .is_ok()
    );
    let unrelated = root.path().join("another");
    fs::hard_link(&dependency, root.path().join("another.d")).unwrap();
    assert!(
        check_owner_sources("native", &output, &sources, |_| Ok(json!([
            unrelated.as_os_str().as_encoded_bytes(),
            [path.as_bytes()]
        ])))
        .is_err()
    );
    let linked = root.path().join("linked");
    std::os::unix::fs::symlink(&dependency, root.path().join("linked.d")).unwrap();
    assert!(
        check_owner_sources("native", &output, &sources, |_| Ok(json!([
            linked.as_os_str().as_encoded_bytes(),
            [path.as_bytes()]
        ])))
        .is_err()
    );
    assert!(check_owner_sources("proof", &output, &[], |_| panic!()).is_err());
    fs::remove_file(dependency).unwrap();
    assert!(check_owner_sources("native", &output, &sources, |_| panic!()).is_err());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn required_proof_collects_the_original_report_library_and_sources() {
    use chrono_host_bootstrap::compiler::OutputContract;
    use chrono_host_bootstrap::proved_build::check_owner_proof;
    use serde_json::json;
    use std::fs;

    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("proof.rlib");
    let dep = root.path().join("proof.rlib.d");
    let report = root.path().join("proof.json");
    let src = root.path().join("root.rs");
    let sources = vec![src.to_str().unwrap().to_owned()];
    let contract = OutputContract {
        version: "version".into(),
        commit: "commit".into(),
    };
    let original = "{ \"compiler\": \"report\" }\n";
    fs::write(&report, original).unwrap();
    fs::write(&dep, b"original manifest").unwrap();
    fs::write(&src, b"original source").unwrap();
    assert!(check_owner_proof(&contract, &report, &output, &sources, |_| panic!()).is_err());
    fs::write(&output, b"").unwrap();
    assert!(check_owner_proof(&contract, &report, &output, &sources, |_| panic!()).is_err());
    fs::write(&output, b"compiled library").unwrap();
    for (rows, expected) in [
        (json!([true]), true),
        (json!([false]), false),
        (json!([null]), false),
        (json!([]), false),
        (json!([true, true]), false),
    ] {
        let admitted = check_owner_proof(&contract, &report, &output, &sources, |request| {
            match request["operation"].as_str() {
                Some("proof-rows") => {
                    assert_eq!(
                        request,
                        json!({"operation":"proof-rows","contract":contract,"rows":[original]})
                    );
                    Ok(rows.clone())
                }
                Some("dependency-matches") if expected => Ok(json!(true)),
                _ => panic!("unexpected protocol call {request}"),
            }
        });
        assert_eq!(admitted.is_ok(), expected);
        if expected {
            assert_eq!(admitted.unwrap(), original);
        }
    }
    fs::remove_file(&dep).unwrap();
    assert!(
        check_owner_proof(&contract, &report, &output, &sources, |_| Ok(json!([true]))).is_err()
    );
    fs::remove_file(&report).unwrap();
    assert!(check_owner_proof(&contract, &report, &output, &sources, |_| panic!()).is_err());
}

#[test]
#[cfg(not(verus_keep_ghost))]
fn collect_binds_the_requested_operation_to_the_observed_streams_and_artifacts() {
    use chrono_host_bootstrap::{compiler::OutputContract, files::sha256, owner_proof::Proof};
    use serde_json::{Value, json};
    use std::{fs, path::Path};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let verifier = root.join("verifier");
    fs::write(&verifier, b"mock compiler").unwrap();
    let scratch = root.join("scratch");
    fs::create_dir(&scratch).unwrap();
    let source = tempfile::Builder::new()
        .prefix("proof_test_")
        .suffix(".rs")
        .tempfile_in(".")
        .unwrap();
    let source_name = source
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let sources = vec![source_name.clone()];
    let contract = OutputContract {
        version: "v".into(),
        commit: "c".into(),
    };
    let collect = |name: &str| {
        Proof::new(
            &root,
            &scratch,
            name,
            verifier.to_str().unwrap(),
            &source_name,
        )
        .unwrap()
        .collect(&contract, &sources, |q| {
            Ok(if q["operation"] == "proof-rows" {
                json!([true])
            } else {
                json!(true)
            })
        })
    };
    let publish = |name: &str, raw: &str| {
        let dir = scratch.join(name);
        fs::create_dir(&dir).unwrap();
        let lib = dir.join("proof.rlib");
        let dep = dir.join("proof.rlib.d");
        let binary = b"independent library";
        let manifest = b"independent dependencies";
        fs::write(&lib, binary).unwrap();
        fs::write(&dep, manifest).unwrap();
        let path = |s: &str| dir.join(s).strip_prefix(&root).unwrap().to_path_buf();
        let stream =
            |p: &Path, data: &[u8]| json!({"path":p,"sha256":sha256(data),"size":data.len()});
        fs::write(dir.join("0.stdout"), raw).unwrap();
        fs::write(dir.join("0.stderr"), b"").unwrap();
        let proc = json!({"operation":name,"tool":"verus","argv":[verifier,"--crate-type=lib","--no-cheating","--no-external-by-default","--num-threads","2","--output-json","--compile",format!("--emit=link={},dep-info={}",lib.display(),dep.display()),source_name,"-o",lib,"-Zchecksum-hash-algorithm=sha256"],"cwd":root,"executable_sha256":sha256(b"mock compiler"),"pid":123,"exit_code":0,"stdout":stream(&path("0.stdout"),raw.as_bytes()),"stderr":stream(&path("0.stderr"),b""),"duration_seconds":1.0});
        let receipt = json!({"process":proc,"library":[sha256(binary),binary.len()],"dependencies":[sha256(manifest),manifest.len()]});
        fs::write(
            dir.join("receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
    };
    let a = "{\"verified\":1}\n";
    let b = "{\"verified\":3}\n";
    publish("product", a);
    publish("tests", b);
    assert_eq!(collect("product").unwrap(), a);
    assert_eq!(collect("tests").unwrap(), b);
    assert!(collect("missing").is_err());
    let prod = scratch.join("product");
    let obs = scratch.join("tests");
    for (field, wrong) in [
        ("0.stdout", b.as_bytes()),
        ("proof.rlib.d", b"changed"),
        ("proof.rlib", b"other library"),
    ] {
        let p = prod.join(field);
        let old = fs::read(&p).unwrap();
        fs::write(&p, wrong).unwrap();
        assert!(collect("product").is_err(), "{field}");
        fs::write(&p, old).unwrap();
    }
    let receipt = prod.join("receipt.json");
    let old = fs::read(&receipt).unwrap();
    fs::copy(obs.join("receipt.json"), &receipt).unwrap();
    assert!(collect("product").is_err());
    let mut mutated: Value = serde_json::from_slice(&old).unwrap();
    mutated["process"]["argv"][9] = json!("different.rs");
    fs::write(&receipt, serde_json::to_vec(&mutated).unwrap()).unwrap();
    assert!(collect("product").is_err());
}

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_sources_require_a_mode_and_a_matching_native_original() {
    use cargo::{OwnerSourceMode, NativeHeader};
    use std::cmp::Ordering::{Less, Equal, Greater};
    proof { reveal_strlit("native"); reveal_strlit("proof"); reveal_strlit("other");
        assert("native"@.len() != "proof"@.len() && "other"@.len() != "native"@.len());
        assert("proof"@[0] != "other"@[0]); }
    expect(cargo::owner_source_mode("native", 0).is_none());
    expect(matches!(cargo::owner_source_mode("native", 1), Some(OwnerSourceMode::Native)));
    expect(matches!(cargo::owner_source_mode("proof", 1), Some(OwnerSourceMode::Proof)));
    expect(matches!(cargo::owner_source_mode("other", 1), Some(OwnerSourceMode::Unknown)));
    expect(matches!(cargo::native_header(Equal, None), NativeHeader::Accept));
    expect(matches!(cargo::native_header(Less, None), NativeHeader::InspectAlias));
    expect(matches!(cargo::native_header(Greater, Some((true, Equal))), NativeHeader::Accept));
    expect(matches!(cargo::native_header(Greater, Some((false, Equal))), NativeHeader::Reject));
    expect(matches!(cargo::native_header(Less, Some((true, Greater))), NativeHeader::Reject));
}
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn an_unbound_cargo_stream_never_reaches_the_verifier() {
    use chrono_host_bootstrap::{cargo_proofs::Replay, files::sha256};
    use serde_json::json;
    let target = json!({"package":"p","package_id":"p","kind":"example","target":"business","manifest":"/h/Cargo.toml","source":"/h/src.rs"});
    let stream = json!({"path":"target/operation/0.stdout","size":1,"sha256":sha256(b"x")});
    let receipt = json!({"packages":["p"],"process":{"operation":"proofs","tool":"cargo-verus","argv":[],"cwd":"/h","executable_sha256":"a","pid":1,"exit_code":0,
        "stdout":stream,"stderr":stream,"duration_seconds":0.1},"inventory":{"p/business":target},"files":{}});
    let original = json!({"schema":"chrono-cargo-verus-root/v1","workspace":"Cargo.toml","target":target,"artifact":{"path":"file","sha256":"a","length":1},
        "dep_info":{"path":"input.d","original":"x"},"report_sha256":sha256(b"{}")});
    let value = json!({"receipt":receipt,"stdout":"y","contract":{"version":"v","commit":"c"},"roots":[{"path":"src.rs","original":original,"sources":{"src.rs":[sha256(b"s"),1]}}]});
    let archive: Replay = serde_json::from_value(value).unwrap();
    let result =
        archive.validate(|_, _| panic!("a changed stdout must be rejected before proof admission"));
    assert_eq!(result.unwrap_err(), "Cargo archive original stream differs");
}
