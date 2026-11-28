#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::{verified_bindings::artifact_identity, verified_cargo as cargo};
#[cfg(verus_keep_ghost)]
#[path = "../../host-bootstrap/src/verified_cargo.rs"]
mod cargo;

use cargo::{Event, State};

#[cfg(verus_keep_ghost)]
#[path = "../../host-bootstrap/src/artifact_identity.rs"]
mod artifact_identity;

use vstd::prelude::*;

#[path = "support/expect.rs"]
mod expectation;
use expectation::expect;

#[cfg(verus_keep_ghost)]
#[path = "registered_methods.rs"]
mod registered_methods_boundaries;

verus! {

fn expect_native_missing(rows: &Vec<cargo::NativeRow>, manifest: &str)
    requires !cargo::missing_native_filenames(rows@, manifest@),
        cargo::cargo_metadata(rows@, manifest@).len() == 0,
{
    let missing = match cargo::select_native_library(rows, manifest) {
        cargo::NativeLibrary::Missing => true,
        _ => false,
    };
    expect(missing);
}

fn expect_native_unique(rows: &Vec<cargo::NativeRow>, manifest: &str, expected: &String)
    requires !cargo::missing_native_filenames(rows@, manifest@),
        cargo::cargo_metadata(rows@, manifest@) == seq![*expected],
{
    let selection = cargo::select_native_library(rows, manifest);
    let unique = match selection {
        cargo::NativeLibrary::Unique(found) => {
            proof {
                let selected = cargo::cargo_metadata(rows@, manifest@);
                assert(selected == seq![found]);
                assert(selected[0] == found);
                assert(selected[0] == *expected);
                assert(found@ == expected@);
            }
            artifact_identity::equal_text(found.as_str(), expected.as_str())
        },
        _ => false,
    };
    expect(unique);
}

fn expect_native_ambiguous(rows: &Vec<cargo::NativeRow>, manifest: &str)
    requires !cargo::missing_native_filenames(rows@, manifest@),
        cargo::cargo_metadata(rows@, manifest@).len() > 1,
{
    let ambiguous = match cargo::select_native_library(rows, manifest) {
        cargo::NativeLibrary::Ambiguous => true,
        _ => false,
    };
    expect(ambiguous);
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn native_selection_ignores_other_targets_and_counts_duplicate_paths() {
    use cargo::{NativeRow, NativeTarget};
    let manifest = "/product/Cargo.toml";
    let metadata = "/custom dir/deps/libunit.rmeta".to_owned();
    let row = |reason: &str, manifest: &str, kinds: &[&str], files| NativeRow {
        reason: reason.to_owned(),
        manifest_path: Some(manifest.to_owned()),
        target: Some(NativeTarget { kind: kinds.iter().map(|kind| kind.to_string()).collect() }),
        filenames: files,
    };
    let mut rows = vec![
        row("compiler-artifact", "/other/Cargo.toml", &["lib"], Some(vec![metadata.clone()])),
        row("compiler-artifact", manifest, &["bin"], Some(vec![metadata.clone()])),
        row("compiler-artifact", manifest, &["lib", "bin"], Some(vec![metadata.clone()])),
        row("compiler-message", manifest, &["lib"], None),
    ];
    expect_native_missing(&rows, manifest);
    rows.push(row("compiler-artifact", manifest, &["lib"], Some(vec![
        "/custom dir/deps/libunit.rlib".to_owned(), metadata.clone(),
    ])));
    expect_native_unique(&rows, manifest, &metadata);
    rows.push(row("compiler-artifact", manifest, &["lib"], Some(vec![metadata.clone()])));
    expect_native_ambiguous(&rows, manifest);
    rows.pop();
    rows.push(row("compiler-artifact", manifest, &["lib"], Some(vec![metadata.clone(), metadata])));
    expect_native_ambiguous(&rows, manifest);
    rows.push(row("compiler-artifact", manifest, &["lib"], None));
    assert!(matches!(cargo::select_native_library(&rows, manifest), cargo::NativeLibrary::InvalidFilenames));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_stem_keeps_the_directory_and_removes_the_leaf_prefix_and_suffix() {
    // lib/libx.rmeta -> lib/x
    let input = [108u8, 105, 98, 47, 108, 105, 98, 120, 46, 114, 109, 101, 116, 97];
    let expected = [108u8, 105, 98, 47, 120];
    let path = vstd::slice::slice_subrange(&input, 0, input.len());
    assert(path@ =~= input@);
    proof {
        assert(path@.skip(8) =~= seq![46u8, 114, 109, 101, 116, 97]);
        let leaf = path@.take(8);
        assert(leaf.subrange(4, 7) =~= seq![108u8, 105, 98]);
        assert forall|i: int| 4 <= i < leaf.len() implies leaf[i] != 47 by {
            assert(i == 4 || i == 5 || i == 6 || i == 7);
        }
        assert(cargo::basename_start(leaf, 4));
    }
    let actual = cargo::native_dep_info_stem(path);
    proof {
        cargo::stem_at_basename(path@, 4);
        assert(actual@ =~= expected@);
    }
    expect(artifact_identity::equal_bytes(actual.as_slice(), &expected));
}

#[cfg(verus_keep_ghost)]
fn source_command_preserves_the_callers_full_list(command: String, mode: String, output: String, source: String)
    requires command@ == "check-compiler-sources"@,
{
    let args = [command, mode, output, source];
    let (mode_arg, output_arg, sources) = cargo::owner_source_arguments(&args).unwrap();
    assert(mode_arg@ == args@[1]@ && output_arg@ == args@[2]@ && sources@ =~= seq![args@[3]]);
    expect(cargo::owner_source_arguments(vstd::slice::slice_subrange(&args, 0, 2)).is_none());
}



#[cfg_attr(not(verus_keep_ghost), test)]
fn required_operation_has_one_successful_report_and_a_nonempty_regular_library() {
    expect(cargo::one_successful_proof(&[Some(true)]));
    expect(!cargo::one_successful_proof(&[]));
    expect(!cargo::one_successful_proof(&[None]));
    expect(!cargo::one_successful_proof(&[Some(false)]));
    expect(!cargo::one_successful_proof(&[Some(true), Some(true)]));
    expect(cargo::nonempty_artifact(true, 1));
    expect(!cargo::nonempty_artifact(true, 0));
    expect(!cargo::nonempty_artifact(false, 5));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn archived_proof_requires_one_success_per_root() {
    expect(cargo::successful_proofs(&[Some(true), Some(true)], 2));
    expect(!cargo::successful_proofs(&[Some(true)], 2));
    expect(!cargo::successful_proofs(&[Some(true), Some(true)], 1));
    expect(!cargo::successful_proofs(&[Some(true), None], 2));
    expect(!cargo::successful_proofs(&[Some(false), Some(true)], 2));
}

#[cfg(verus_keep_ghost)]
fn archived_source_requires_a_unique_exact_checksum(
    digest: &str, other_digest: &str, length: &str, other_length: &str,
)
    requires digest@ != other_digest@, length@ != other_length@,
{
    expect(cargo::archive_source_matches(true, digest, digest, length, length));
    expect(!cargo::archive_source_matches(false, digest, digest, length, length));
    expect(!cargo::archive_source_matches(true, digest, other_digest, length, length));
    expect(!cargo::archive_source_matches(true, digest, digest, other_length, length));
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn native_source_check_requires_every_declared_byte_path() {
    let src = "a".to_owned();
    let absent = "ab".to_owned();
    let mut name = Vec::new();
    name.extend_from_slice(src.as_str().as_bytes());
    let observed = vec![name];
    let same = vec![src.clone(),src.clone()];
    assert(observed@[0]@ == vstd::utf8::encode_utf8(src@));
    assert(cargo::compiler_has_path(observed@,vstd::utf8::encode_utf8(src@)));
    assert forall|i:int| 0 <= i < same.len() implies
        cargo::compiler_has_path(observed@,vstd::utf8::encode_utf8((#[trigger] same@[i])@)) by {
        assert(i == 0 || i == 1);
    }
    expect(cargo::required_compiler_paths(&observed,&same));
    proof {
        reveal_strlit("a"); reveal_strlit("ab");
        vstd::utf8::encode_utf8_decode_utf8(src@);
        vstd::utf8::encode_utf8_decode_utf8(absent@);
        assert(vstd::utf8::encode_utf8(src@) != vstd::utf8::encode_utf8(absent@));
    }
    expect(!cargo::required_compiler_paths(&observed,&vec![src,absent]));
    expect(!cargo::required_compiler_paths(&observed,&vec![]));
}

proof fn library_literal_bytes()
    ensures vstd::utf8::encode_utf8("d/libn-a.rlib"@) == seq![100u8,47,108,105,98,110,45,97,46,114,108,105,98],
        vstd::utf8::encode_utf8("d/libn-a.rmeta"@) == seq![100u8,47,108,105,98,110,45,97,46,114,109,101,116,97],
        vstd::utf8::encode_utf8("d/libn-b.rmeta"@) == seq![100u8,47,108,105,98,110,45,98,46,114,109,101,116,97],
{
        reveal_strlit("d/libn-a.rlib"); reveal_strlit("d/libn-a.rmeta");
        reveal_strlit("d/libn-b.rmeta");
        assert('d' as u32 == 100 && '/' as u32 == 47 && 'l' as u32 == 108 && 'i' as u32 == 105 && 'b' as u32 == 98 && 'n' as u32 == 110 && '-' as u32 == 45 && 'a' as u32 == 97 && '.' as u32 == 46 && 'r' as u32 == 114 && 'm' as u32 == 109 && 'e' as u32 == 101 && 't' as u32 == 116);
        assert(vstd::utf8::encode_scalar(100) == seq![100u8]) by (compute);
        assert(vstd::utf8::encode_scalar(47) == seq![47u8]) by (compute);
        assert(vstd::utf8::encode_scalar(108) == seq![108u8]) by (compute);
        assert(vstd::utf8::encode_scalar(105) == seq![105u8]) by (compute);
        assert(vstd::utf8::encode_scalar(98) == seq![98u8]) by (compute);
        assert(vstd::utf8::encode_scalar(110) == seq![110u8]) by (compute);
        assert(vstd::utf8::encode_scalar(45) == seq![45u8]) by (compute);
        assert(vstd::utf8::encode_scalar(97) == seq![97u8]) by (compute);
        assert(vstd::utf8::encode_scalar(46) == seq![46u8]) by (compute);
        assert(vstd::utf8::encode_scalar(114) == seq![114u8]) by (compute);
        assert(vstd::utf8::encode_scalar(109) == seq![109u8]) by (compute);
        assert(vstd::utf8::encode_scalar(101) == seq![101u8]) by (compute);
        assert(vstd::utf8::encode_scalar(116) == seq![116u8]) by (compute);
        reveal_with_fuel(vstd::utf8::encode_utf8, 24);
        assert(vstd::utf8::encode_utf8("d/libn-a.rlib"@) =~= seq![100u8,47,108,105,98,110,45,97,46,114,108,105,98]);
        assert(vstd::utf8::encode_utf8("d/libn-a.rmeta"@) =~= seq![100u8,47,108,105,98,110,45,97,46,114,109,101,116,97]);
        assert(vstd::utf8::encode_utf8("d/libn-b.rmeta"@) =~= seq![100u8,47,108,105,98,110,45,98,46,114,109,101,116,97]);
 }

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_library_paths_require_one_directory_and_one_fingerprint() {
    let mut files = vec!["d/libn-a.rlib".to_string(), "d/libn-a.rmeta".to_string()];
    proof {
        library_literal_bytes();
        assert(cargo::library_hash(seq![100u8], seq![110u8], files@) =~= seq![97u8]);
        let prefix = cargo::library_prefix(seq![100u8], seq![110u8]);
        assert(prefix + seq![97u8] + seq![46u8,114,108,105,98] =~= vstd::utf8::encode_utf8(files[0]@));
        assert(prefix + seq![97u8] + seq![46u8,114,109,101,116,97] =~= vstd::utf8::encode_utf8(files[1]@));
        assert(cargo::library_shape(seq![100u8], seq![110u8], files@, false));
    }
    let directory = [100u8];
    let name = [110u8];
    proof { assert(directory@ == seq![100u8]); assert(name@ == seq![110u8]);
        assert(cargo::library_shape(directory@, name@, files@, false)); }
    let matched = cargo::compiler_library_paths(directory.as_slice(), name.as_slice(), &files, false);
    expect(matched.is_some());
    let output = matched.unwrap();
    let expected = [100u8, 47, 110, 45, 97, 46, 100];
    proof { assert(output.dependency@ =~= expected@); }
    let correct = artifact_identity::equal_bytes(output.dependency.as_slice(), &expected);
    expect(correct);
    expect(!cargo::compiler_library_paths(&[100u8], &[110u8], &files, true).is_some());
    expect(!cargo::compiler_library_paths(&[120u8], &[110u8], &files, false).is_some());
    files = vec!["d/libn-a.rlib".to_string(), "d/libn-b.rmeta".to_string()];
    proof { library_literal_bytes(); }
    expect(!cargo::compiler_library_paths(&[100u8], &[110u8], &files, false).is_some());
    let empty: Vec<String> = Vec::new();
    expect(!cargo::compiler_library_paths(&[100u8], &[110u8], &empty, false).is_some());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn library_names_and_fingerprints_cannot_escape_the_declared_output_directory() {
    expect(cargo::library_name(&[97u8, 90, 48, 95]));
    expect(!cargo::library_name(&[]));
    expect(!cargo::library_name(&[97u8, 47, 98]));
    expect(!cargo::library_name(&[46u8, 46]));
    expect(!cargo::library_name(&[0u8]));
    expect(cargo::fingerprint(&[48u8, 57, 97, 102]));
    expect(!cargo::fingerprint(&[]));
    expect(!cargo::fingerprint(&[65u8]));
    expect(!cargo::fingerprint(&[48u8, 47]));
    let name = [97u8,45,48,102,46,100];
    let prefix = [97u8,45];
    proof {
        assert(name@.subrange(0,2) =~= prefix@);
        assert(name@.subrange(2,4) =~= seq![48u8,102]);
        assert(cargo::dependency_name(name@, prefix@));
    }
    expect(cargo::selected_dependency(name.as_slice(), prefix.as_slice()));
    expect(!cargo::selected_dependency(&[97u8,45,46,100], &[97u8,45]));
    let bad = [97u8,45,65,46,100];
    proof {
        assert(bad@.subrange(2,3) =~= seq![65u8]);
        assert(!cargo::fingerprint_byte(bad@.subrange(2,3)[0]));
        assert(!cargo::dependency_name(bad@, prefix@));
    }
    expect(!cargo::selected_dependency(bad.as_slice(), prefix.as_slice()));
    expect(!cargo::selected_dependency(&[97u8,45,48,46,100,120], &[97u8,45]));
    expect(!cargo::selected_dependency(&[98u8,45,48,46,100], &[97u8,45]));
    expect(!cargo::selected_dependency(&[97u8,45,48,46,100], &[]));
    expect(cargo::all_present(&vec![true, true]));
    expect(!cargo::all_present(&vec![true, false]));
    expect(!cargo::all_present(&Vec::new()));
    expect(cargo::output_inventory_within_bound(4096));
    expect(!cargo::output_inventory_within_bound(4097));
    expect(!cargo::retain_attempt(true, false));
    expect(cargo::retain_attempt(true, true));
    expect(cargo::retain_attempt(false, false));
    expect(cargo::retain_attempt(false, true));
    expect(cargo::bounded_original(true, 8_388_608));
    expect(!cargo::bounded_original(true, 8_388_609));
    expect(!cargo::bounded_original(false, 0));
    expect(cargo::content_matches("x", 0, "x", 0));
    expect(!cargo::content_matches("x", 0, "x", 1));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registration_and_recovery_preserve_the_declared_source_and_retry_bound() {
    use cargo::{registered_sources, recovery, Recovery};
    proof { reveal_strlit("chrono-proved-build/v1"); reveal_strlit("other");
        reveal_strlit("s"); reveal_strlit("x");
        assert("chrono-proved-build/v1"@.len() != "other"@.len());
        assert("s"@[0] != "x"@[0]); }
    let sources = vec!["s".to_string()];
    expect(registered_sources("chrono-proved-build/v1", "s", &sources));
    expect(!registered_sources("chrono-proved-build/v1", "x", &sources));
    expect(!registered_sources("other", "s", &sources));
    expect(!registered_sources("chrono-proved-build/v1", "s", &Vec::new()));
    expect(match recovery(false, false, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, false, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, true, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(false, true, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, true, false) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, true, true) { Recovery::Continue => true, _ => false });
    expect(match recovery(true, false, true) { Recovery::Rebuild => true, _ => false });
    expect(match recovery(true, false, false) { Recovery::Reject => true, _ => false });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_original_records_reject_errors_trailing_records_and_missing_outputs() {
    use cargo::{Artifact, Target, TargetIdentity, BuildTargets, Record, Cargo, complete_records};
    proof {
        reveal_strlit("p"); reveal_strlit("s"); reveal_strlit("n"); reveal_strlit("b");
        reveal_strlit("lib"); reveal_strlit("bin"); reveal_strlit("out");
        reveal_strlit("error"); reveal_strlit("warning"); reveal_strlit("note"); reveal_strlit("help");
        assert("lib"@[0] != "bin"@[0]);
    }
    let targets = BuildTargets {
        library: TargetIdentity { manifest: "p".to_string(), source: "s".to_string(), name: "n".to_string() },
        binary: TargetIdentity { manifest: "p".to_string(), source: "s".to_string(), name: "b".to_string() },
        published: "out".to_string(),
    };
    let mut records = vec![
        Record::Cargo(Cargo::Artifact(Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
            target: Target { name: "n".to_string(), src_path: "s".to_string(), kind: vec!["lib".to_string()], crate_types: vec!["lib".to_string()] },
            filenames: Vec::new(), executable: None, fresh: true })),
        Record::Cargo(Cargo::Artifact(Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
            target: Target { name: "b".to_string(), src_path: "s".to_string(), kind: vec!["bin".to_string()], crate_types: vec!["bin".to_string()] },
            filenames: vec!["out".to_string()], executable: Some("out".to_string()), fresh: true })),
    ];
    expect(!complete_records(&records, &targets));
    records.push(Record::Cargo(Cargo::Finished { success: true }));
    proof { reveal_with_fuel(cargo::trace, 5); }
    expect(complete_records(&records, &targets));
    records.push(Record::Cargo(Cargo::Finished { success: true }));
    proof { reveal_with_fuel(cargo::trace, 6); }
    expect(!complete_records(&records, &targets));
    let empty = Vec::new();
    expect(!complete_records(&empty, &targets));
    let errors = vec![Record::Cargo(Cargo::Message { message: cargo::Message { level: "error".to_string() } }),
        Record::Cargo(Cargo::Finished { success: true })];
    expect(!complete_records(&errors, &targets));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn retained_proofs_bind_the_current_inputs_full_metadata_and_original_success() {
    let bytes = [1u8, 2, 3];
    let changed = [1u8, 2, 4];
    proof {
        reveal_strlit("chrono-library-proof/v1"); reveal_strlit("wrong");
        reveal_strlit("input"); reveal_strlit("other");
        assert("input"@[0] != "other"@[0]);
        assert("chrono-library-proof/v1"@.len() != "wrong"@.len());
    }
    expect(cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, false, &bytes, &bytes));
    expect(!cargo::retained_header("wrong", "input", "input", 0, false, &bytes, &bytes));
    expect(!cargo::retained_header("chrono-library-proof/v1", "input", "other", 0, false, &bytes, &bytes));
    expect(!cargo::retained_header("chrono-library-proof/v1", "input", "input", 1, false, &bytes, &bytes));
    expect(!cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, true, &bytes, &bytes));
    expect(!cargo::retained_header("chrono-library-proof/v1", "input", "input", 0, false, &bytes, &changed));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn artifact_selection_requires_the_entire_declared_target_identity() {
    use cargo::{Artifact, Target, artifact_selected};
    proof {
        reveal_strlit("p"); reveal_strlit("s"); reveal_strlit("n"); reveal_strlit("other");
        reveal_strlit("lib"); reveal_strlit("bin");
        assert("lib"@[0] != "bin"@[0]);
        assert("p"@.len() != "other"@.len());
        assert("s"@.len() != "other"@.len());
        assert("n"@.len() != "other"@.len());
    }
    let mut artifact = Artifact { package_id: "id".to_string(), manifest_path: "p".to_string(),
        target: Target { name: "n".to_string(), src_path: "s".to_string(),
            kind: vec!["lib".to_string()], crate_types: vec!["lib".to_string()] },
        filenames: Vec::new(), executable: None, fresh: false };
    expect(artifact_selected(&artifact, "p", "s", "n", "lib"));
    expect(!artifact_selected(&artifact, "other", "s", "n", "lib"));
    expect(!artifact_selected(&artifact, "p", "other", "n", "lib"));
    expect(!artifact_selected(&artifact, "p", "s", "other", "lib"));
    expect(!artifact_selected(&artifact, "p", "s", "n", "bin"));
    artifact.target.kind.push("lib".to_string());
    expect(!artifact_selected(&artifact, "p", "s", "n", "lib"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_error_diagnostics_cannot_be_hidden_by_a_successful_terminal_record() {
    proof {
        reveal_strlit("warning"); reveal_strlit("error"); reveal_strlit("failure-note");
        reveal_strlit("note"); reveal_strlit("help");
        assert("error"@.len() != "note"@.len()); assert("error"@.len() != "help"@.len());
        assert("failure-note"@.len() != "warning"@.len()); assert("failure-note"@.len() != "note"@.len()); assert("failure-note"@.len() != "help"@.len());
        assert("warning"@.len() != "error"@.len());
        assert("warning"@.len() != "failure-note"@.len());
    }
    expect(cargo::diagnostic_rejected("error"));
    expect(cargo::diagnostic_rejected("failure-note"));
    expect(!cargo::diagnostic_rejected("warning"));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn an_empty_or_unfinished_build_never_proves_a_library() {
    let state = State::new();
    expect(!state.accepted());
    let selected = state.step(Event::Library { fresh: false, matches: true });
    expect(selected.is_err());
    let finished = state.step(Event::Finished { success: true });
    expect(finished.is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn a_cold_build_requires_its_own_proof_library_and_final_binary() {
    let state = State::new();
    let proof = state.step(Event::Proof { accepted: true });
    expect(proof.is_ok());
    let library = proof.unwrap().step(Event::Library { fresh: false, matches: true });
    expect(library.is_ok());
    let binary = library.unwrap().step(Event::Binary { matches: true });
    expect(binary.is_ok());
    let finished = binary.unwrap().step(Event::Finished { success: true });
    expect(finished.is_ok());
    expect(finished.unwrap().accepted());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn fresh_artifacts_require_original_proof_and_matching_bytes() {
    let state = State::new();
    let missing = state.step(Event::Library { fresh: true, matches: false });
    expect(missing.is_err());
    let restored = state.step(Event::Library { fresh: true, matches: true });
    expect(restored.is_ok());
    let binary = restored.unwrap().step(Event::Binary { matches: true });
    expect(binary.is_ok());
    let finished = binary.unwrap().step(Event::Finished { success: true });
    expect(finished.is_ok());
    expect(finished.unwrap().accepted());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn parallel_dependency_artifacts_can_arrive_between_selected_proof_and_library() {
    let state = State::new().step(Event::Proof { accepted: true }).unwrap();
    let interleaved = state.step(Event::OtherArtifact);
    expect(interleaved.is_ok());
    let library = interleaved.unwrap().step(Event::Library { fresh: false, matches: true }).unwrap();
    let binary = library.step(Event::Binary { matches: true }).unwrap();
    expect(binary.step(Event::Finished { success: true }).unwrap().accepted());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn duplicate_proofs_libraries_and_terminal_records_are_errors() {
    let proof = State::new().step(Event::Proof { accepted: true }).unwrap();
    expect(proof.step(Event::Proof { accepted: true }).is_err());
    expect(proof.step(Event::Binary { matches: true }).is_err());
    expect(proof.step(Event::Library { fresh: true, matches: true }).is_err());
    let library = proof.step(Event::Library { fresh: false, matches: true }).unwrap();
    expect(library.step(Event::Library { fresh: true, matches: true }).is_err());
    expect(library.step(Event::Finished { success: true }).is_err());
    let binary = library.step(Event::Binary { matches: true }).unwrap();
    expect(binary.step(Event::Binary { matches: true }).is_err());
    let done = binary.step(Event::Finished { success: true }).unwrap();
    expect(done.step(Event::Finished { success: true }).is_err());
    expect(done.step(Event::Note).is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn report_failure_diagnostic_errors_and_mismatched_artifacts_reject() {
    let initial = State::new();
    expect(initial.step(Event::Proof { accepted: false }).is_err());
    expect(initial.step(Event::Error).is_err());
    expect(initial.step(Event::Binary { matches: true }).is_err());
    let proof = initial.step(Event::Proof { accepted: true }).unwrap();
    expect(proof.step(Event::Library { fresh: false, matches: false }).is_err());
    let library = proof.step(Event::Library { fresh: false, matches: true }).unwrap();
    expect(library.step(Event::Binary { matches: false }).is_err());
    let binary = library.step(Event::Binary { matches: true }).unwrap();
    expect(binary.step(Event::Finished { success: false }).is_err());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn serial_build_binds_each_immediately_preceding_report_to_one_required_library() {
    use cargo::SerialEvent::*;
    let mut state = cargo::SerialProofs::new(2);
    expect(state.accept(Note));
    expect(state.accept(Artifact { selected: None, fresh: true }));
    expect(!state.accept(Artifact { selected: Some(0), fresh: false }));
    expect(state.accept(Proof));
    expect(!state.accept(Proof));
    expect(!state.accept(Note));
    expect(!state.accept(Artifact { selected: None, fresh: false }));
    expect(!state.accept(Artifact { selected: Some(2), fresh: false }));
    expect(!state.accept(Artifact { selected: Some(1), fresh: true }));
    expect(!state.accept(Finished { success: true }));
    expect(state.accept(Artifact { selected: Some(1), fresh: false }));
    expect(state.accept(Proof));
    expect(!state.accept(Artifact { selected: Some(1), fresh: false }));
    expect(state.accept(Artifact { selected: Some(0), fresh: false }));
    expect(!state.complete());
    expect(!state.accept(Error));
    expect(!state.accept(Finished { success: false }));
    expect(state.accept(Finished { success: true }));
    assert forall|i:int| 0 <= i < 2 implies state.bound()[i] by { assert(i == 0 || i == 1); }
    expect(state.complete());
    expect(!state.accept(Proof));
    expect(!state.accept(Finished { success: true }));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn serial_build_with_missing_or_empty_inventory_has_no_complete_result() {
    use cargo::SerialEvent::*;
    let mut state = cargo::SerialProofs::new(2);
    expect(state.accept(Proof));
    expect(state.accept(Artifact { selected: Some(0), fresh: false }));
    expect(state.accept(Finished { success: true }));
    assert(!state.bound()[1]);
    expect(!state.complete());
    let mut empty = cargo::SerialProofs::new(0);
    expect(empty.accept(Finished { success: true }));
    expect(!empty.complete());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn serial_library_requires_package_manifest_source_name_kind_and_a_single_lib_crate_type() {
    use cargo::{Artifact, Target, library_selected};
    proof {
        reveal_strlit("lib"); reveal_strlit("rlib"); reveal_strlit("wrong");
        reveal_strlit("p"); reveal_strlit("m"); reveal_strlit("s"); reveal_strlit("n");
        assert("lib"@.len() != "rlib"@.len());
        assert("wrong"@.len() != "p"@.len());
        assert("wrong"@.len() != "m"@.len());
        assert("wrong"@.len() != "s"@.len());
        assert("wrong"@.len() != "n"@.len());
    }
    let mut a = Artifact { package_id: "p".to_string(), manifest_path: "m".to_string(),
        target: Target { name: "n".to_string(), src_path: "s".to_string(),
            kind: vec!["rlib".to_string()], crate_types: vec!["lib".to_string()] },
        filenames: Vec::new(), executable: None, fresh: false };
    expect(library_selected(&a, "p", "m", "s", "n", "rlib"));
    expect(!library_selected(&a, "wrong", "m", "s", "n", "rlib"));
    expect(!library_selected(&a, "p", "wrong", "s", "n", "rlib"));
    expect(!library_selected(&a, "p", "m", "wrong", "n", "rlib"));
    expect(!library_selected(&a, "p", "m", "s", "wrong", "rlib"));
    expect(!library_selected(&a, "p", "m", "s", "n", "lib"));
    a.target.crate_types = vec!["rlib".to_string()];
    expect(!library_selected(&a, "p", "m", "s", "n", "rlib"));
    a.target.crate_types.push("lib".to_string());
    expect(!library_selected(&a, "p", "m", "s", "n", "rlib"));
}

}
