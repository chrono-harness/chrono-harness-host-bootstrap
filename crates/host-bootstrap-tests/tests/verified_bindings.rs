#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::verified_bindings as policy;
#[cfg(verus_keep_ghost)]
#[path = "../../host-bootstrap/src/verified_bindings.rs"]
mod policy;

use policy::Executable;

use vstd::prelude::*;
#[path = "support/expect.rs"]
mod expectation;
use expectation::expect;

#[path = "proof_support/host-bootstrap/construction.rs"]
mod construction_tests;

#[path = "proof_support/host-bootstrap/owner.rs"]
mod stage_tests;

#[path = "proof_support/host-bootstrap/original.rs"]
mod original_tests;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_directory_decisions_require_time_identity_and_exclusive_use() {
    use policy::native_policy::*;
    expect(candidate(true,false));
    expect(!candidate(true,true));
    expect(!candidate(false,false));
    expect(time_ready(None,None));
    expect(!time_ready(Some(8),None));
    expect(!time_ready(Some(0),Some(8)));
    expect(!time_ready(Some(8),Some(7)));
    expect(time_ready(Some(8),Some(8)));
    expect(time_ready(Some(8),Some(9)));
    expect(marker_owned(true,false,false));
    expect(marker_owned(false,true,false));
    expect(marker_owned(false,false,true));
    expect(!marker_owned(false,false,false));
    expect(removable(true,true,true));
    expect(!removable(false,true,true));
    expect(!removable(true,false,true));
    expect(!removable(true,true,false));
}


#[cfg_attr(not(verus_keep_ghost), test)]
fn install_uses_an_available_manager_and_rejects_storage_in_checkout() {
    let present = policy::ci_manager(true, false);
    let hosted = policy::ci_manager(false, true);
    let local = policy::ci_manager(false, false);
    let bad = policy::ci_storage(true);
    let good = policy::ci_storage(false);
    assert(present@ == "existing"@ && hosted@ == "platform"@ && local@ == "unavailable"@);
    assert(bad@ == "reject"@ && good@ == "accept"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn install_selects_exact_platforms_and_preserves_declared_link_behavior() {
    proof {
        reveal_strlit("Darwin/arm64"); reveal_strlit("Linux/x86_64"); reveal_strlit("other");
        assert("Darwin/arm64"@[0] != "Linux/x86_64"@[0]);
        assert("other"@.len() != "Darwin/arm64"@.len() && "other"@.len() != "Linux/x86_64"@.len());
    }
    let darwin = policy::ci_mise_asset("Darwin/arm64");
    let linux = policy::ci_mise_asset("Linux/x86_64");
    let other = policy::ci_mise_asset("other");
    let keep = policy::ci_link(true, true);
    let replace = policy::ci_link(true, false);
    let create = policy::ci_link(false, false);
    assert(darwin.unwrap()@ == "macos-arm64"@ && linux.unwrap()@ == "linux-x64"@ && other.is_none());
    assert(keep@ == "keep"@ && create@ == "create"@ && replace@ == "replace"@);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn archive_reuse_requires_the_complete_declared_digest() {
    broadcast use vstd::array::group_array_axioms;
    let zeros = [48u8; 64];
    let mut changed = [48u8; 64];
    changed[63] = 49;
    let short = [48u8; 63];
    let upper = [65u8; 64];
    proof {
        assert forall|i: int| 0 <= i < zeros@.len()
            implies policy::hash_byte(zeros@[i]) by { assert(zeros@[i] == 48); }
        assert(changed@[63] != zeros@[63]);
        assert(upper@[0] == 65);
    }
    let hit = policy::archive_matches(&zeros, &zeros);
    let corrupt = policy::archive_matches(&changed, &zeros);
    let partial = policy::archive_matches(&short, &zeros);
    let invalid_length = policy::archive_matches(&short, &short);
    let invalid_hex = policy::archive_matches(&upper, &upper);
    expect(hit && !corrupt && !partial && !invalid_length && !invalid_hex);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn startup_names_modes_and_headers_use_exact_boundaries() {
    proof {
        reveal_strlit("chrono-host-startup/v1"); reveal_strlit("other");
        assert("chrono-host-startup/v1"@.len() != "other"@.len());
    }
    let schema = policy::startup_schema("chrono-host-startup/v1", "chrono-host-startup/v1");
    let changed = policy::startup_schema("other", "chrono-host-startup/v1");
    let empty = policy::startup_name(&[]);
    let allowed = policy::startup_name(&[65u8, 90, 97, 122, 48, 57, 95, 45]);
    let slash = policy::startup_name(&[65u8, 47]);
    let unicode = policy::startup_name(&[195u8, 169]);
    let nul = policy::startup_name(&[0u8]);
    let newline = policy::startup_name(&[10u8]);
    expect(schema && !changed && !empty && allowed && !slash && !unicode && !nul && !newline);
    let executable = policy::startup_mode(493);
    let lowest = policy::startup_mode(1);
    let highest = policy::startup_mode(511);
    let data_file = policy::startup_mode(420);
    let zero = policy::startup_mode(0);
    let special = policy::startup_mode(512);
    let maximum = policy::startup_mode(4294967295u32);
    expect(executable && lowest && highest && data_file && !zero && !special && !maximum);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_headers_require_success_identity_and_no_import_or_error() {
    proof {
        reveal_strlit("chrono-bootstrap-result/v1"); reveal_strlit("bad");
        reveal_strlit("config"); reveal_strlit("other");
        assert("config"@.len() != "other"@.len());
        assert("chrono-bootstrap-result/v1"@.len() != "bad"@.len());
    }
    let good = policy::original_header("chrono-bootstrap-result/v1", true, "config", "config", true, false, false);
    let schema = policy::original_header("bad", true, "config", "config", true, false, false);
    let failed = policy::original_header("chrono-bootstrap-result/v1", false, "config", "config", true, false, false);
    let config = policy::original_header("chrono-bootstrap-result/v1", true, "other", "config", true, false, false);
    let host = policy::original_header("chrono-bootstrap-result/v1", true, "config", "config", false, false, false);
    let imported = policy::original_header("chrono-bootstrap-result/v1", true, "config", "config", true, true, false);
    let error = policy::original_header("chrono-bootstrap-result/v1", true, "config", "config", true, false, true);
    expect(good && !schema && !failed && !config && !host && !imported && !error);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_operations_and_streams_compare_exact_fields_and_terminal_status() {
    proof {
        reveal_strlit("cargo"); reveal_strlit("rustc"); reveal_strlit("arg"); reveal_strlit("changed");
        reveal_strlit("host"); reveal_strlit("elsewhere"); reveal_strlit("hash"); reveal_strlit("different");
        assert("cargo"@[0] != "rustc"@[0]);
        assert("arg"@.len() != "changed"@.len());
        assert("host"@.len() != "elsewhere"@.len());
        assert("hash"@.len() != "different"@.len());
    }
    let mut argv = Vec::new(); argv.push("arg".to_string());
    let mut other = Vec::new(); other.push("changed".to_string());
    let empty: Vec<String> = Vec::new();
    proof {
        assert(policy::same_name_list(argv@, argv@));
        assert(argv@[0]@.len() == 3 && other@[0]@.len() == 7);
        assert(!policy::same_name_list(argv@, other@));
        assert(argv@.len() == 1 && empty@.len() == 0);
    }
    let good = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "host", "host", 1, 0);
    let maximum = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "host", "host", 4294967295u32, 0);
    let tool = policy::operation_record_matches("rustc", "cargo", &argv, &argv, "host", "host", 1, 0);
    let argument = policy::operation_record_matches("cargo", "cargo", &other, &argv, "host", "host", 1, 0);
    let missing = policy::operation_record_matches("cargo", "cargo", &empty, &argv, "host", "host", 1, 0);
    let cwd = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "elsewhere", "host", 1, 0);
    let pid = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "host", "host", 0, 0);
    let failed = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "host", "host", 1, 17);
    let signal = policy::operation_record_matches("cargo", "cargo", &argv, &argv, "host", "host", 1, -9);
    let stream = policy::stream_matches("hash", "hash", 0, 0);
    let large = policy::stream_matches("hash", "hash", 18446744073709551615u64, 18446744073709551615u64);
    let digest = policy::stream_matches("different", "hash", 0, 0);
    let size = policy::stream_matches("hash", "hash", 1, 0);
    expect(good && maximum && !tool && !argument && !missing && !cwd && !pid && !failed && !signal);
    expect(stream && large && !digest && !size);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_source_requires_both_exact_clean_observations_and_exposed_identity() {
    proof { reveal_strlit("c"); reveal_strlit("cc"); reveal_strlit("t"); reveal_strlit("tt"); }
    let expected = source_observation("c", "t", false, None);
    let clean = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", false, None));
    let good = policy::original_source(&clean, &expected);
    expect(good);
    let changed = policy::source_identity(source_observation("c", "t", false, None), source_observation("cc", "tt", false, None));
    proof {
        assert(changed.before.commit.unwrap()@.len() == 1);
        assert(changed.after.commit.unwrap()@.len() == 2);
        assert(!policy::source_observations_same(&changed.before, &changed.after));
    }
    let drift = policy::original_source(&changed, &expected);
    expect(!drift);
    let dirty = policy::source_identity(source_observation("c", "t", true, None), source_observation("c", "t", true, None));
    let unclean = policy::original_source(&dirty, &expected);
    expect(!unclean);
    let error = source_observation("c", "t", false, Some("failed".to_string()));
    let failed = policy::original_source(&clean, &error);
    expect(!failed);
    let mut forged = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", false, None));
    forged.after.dirty = Some(true);
    let bad_after = policy::original_source(&forged, &expected);
    expect(!bad_after);
    forged.after.dirty = Some(false);
    forged.commit = None;
    let hidden = policy::original_source(&forged, &expected);
    expect(!hidden);
    let mut incomplete = source_observation("c", "t", false, None);
    incomplete.commit = None;
    let missing = policy::original_source(&clean, &incomplete);
    expect(good && !drift && !unclean && !failed && !bad_after && !hidden && !missing);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn immutable_producer_requires_two_full_lowercase_git_identities() {
    broadcast use vstd::array::group_array_axioms;
    let full = [49u8; 40];
    let sha256 = [49u8; 64];
    let zero = [48u8; 40];
    let short = [49u8; 39];
    let long = [49u8; 41];
    let mut upper = [49u8; 40]; upper[39] = 65;
    proof {
        assert(full@.len() == 40 && short@.len() == 39 && long@.len() == 41);
        assert forall|i: int| 0 <= i < full@.len() implies full@[i] == 49 by { }
        assert(full@[0] != 48);
        assert forall|i: int| 0 <= i < sha256@.len() implies sha256@[i] == 49 by { }
        assert(sha256@.len() == 64 && sha256@[0] != 48);
        assert forall|i: int| 0 <= i < zero@.len() implies zero@[i] == 48 by { }
        assert(upper@[39] == 65);
    }
    let good = policy::immutable_producer(&full, &full);
    let sha256_identity = policy::immutable_producer(&sha256, &sha256);
    let zero_identity = policy::immutable_producer(&zero, &full);
    let commit = policy::immutable_producer(&short, &full);
    let tree = policy::immutable_producer(&full, &long);
    let uppercase_commit = policy::immutable_producer(&upper, &full);
    let uppercase_tree = policy::immutable_producer(&full, &upper);
    expect(good && sha256_identity && !zero_identity && !commit && !tree && !uppercase_commit && !uppercase_tree);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn construction_success_requires_the_complete_ordered_execution_installation_and_owner_stage() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("tool");
        assert("a"@.len() != "bb"@.len());
    }
    let mut expected = Vec::new(); expected.push("a".to_string()); expected.push("bb".to_string());
    let mut actual = Vec::new(); actual.push("a".to_string()); actual.push("bb".to_string());
    let mut reordered = Vec::new(); reordered.push("bb".to_string()); reordered.push("a".to_string());
    let mut installed = Vec::new(); installed.push("tool".to_string());
    let missing: Vec<String> = Vec::new();
    let mut codes = Vec::new(); codes.push(0i32); codes.push(0i32);
    let mut failed_codes = Vec::new(); failed_codes.push(0i32); failed_codes.push(17i32);
    let mut short_codes = Vec::new(); short_codes.push(0i32);
    proof {
        assert forall|i: int| 0 <= i < expected@.len() implies expected@[i]@ == actual@[i]@ by {
            assert(i == 0 || i == 1);
        }
        assert(policy::same_name_list(expected@, actual@));
        assert(expected@[0]@.len() == 1 && reordered@[0]@.len() == 2);
        assert(!policy::same_name_list(expected@, reordered@));
        assert forall|i: int| 0 <= i < codes@.len() implies codes@[i] == 0 by {
            assert(i == 0 || i == 1);
        }
        assert(failed_codes@[1] == 17 && failed_codes@.len() == 2);
        assert(codes@.len() == actual@.len() && short_codes@.len() != actual@.len());
        assert(policy::same_name_list(installed@, installed@));
        assert(installed@.len() == 1 && missing@.len() == 0);
        assert(!policy::same_name_list(installed@, missing@));
    }
    let complete = policy::construction_complete(&expected, &actual, &codes, &installed, &installed, true, false, false);
    let stage = policy::construction_complete(&expected, &actual, &codes, &installed, &installed, true, true, true);
    let unordered = policy::construction_complete(&expected, &reordered, &codes, &installed, &installed, true, false, false);
    let failed = policy::construction_complete(&expected, &actual, &failed_codes, &installed, &installed, true, false, false);
    let no_code = policy::construction_complete(&expected, &actual, &short_codes, &installed, &installed, true, false, false);
    let no_install = policy::construction_complete(&expected, &actual, &codes, &installed, &missing, true, false, false);
    let no_selection = policy::construction_complete(&expected, &actual, &codes, &installed, &installed, false, false, false);
    let no_stage = policy::construction_complete(&expected, &actual, &codes, &installed, &installed, true, true, false);
    let extra_stage = policy::construction_complete(&expected, &actual, &codes, &installed, &installed, true, false, true);
    expect(complete && stage && !unordered && !failed && !no_code && !no_install && !no_selection && !no_stage && !extra_stage);
    let passed = policy::construction_status(false, true);
    let interrupted = policy::construction_status(false, false);
    let errored = policy::construction_status(true, true);
    let missing_failed = policy::construction_status(true, false);
    expect(matches!(passed, policy::BuildStatus::Passed) && matches!(interrupted, policy::BuildStatus::Failed) && matches!(errored, policy::BuildStatus::Failed) && matches!(missing_failed, policy::BuildStatus::Failed));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn profile_sequences_keep_empty_subsets_but_reject_empty_or_duplicate_operation_names() {
    proof {
        reveal_strlit("a"); reveal_strlit("bb"); reveal_strlit("ccc"); reveal_strlit("");
        assert("a"@.len() == 1 && "bb"@.len() == 2 && "ccc"@.len() == 3);
    }
    let empty: Vec<String> = Vec::new();
    let mut names = Vec::new(); names.push("a".to_string()); names.push("bb".to_string());
    let mut duplicate = Vec::new(); duplicate.push("a".to_string()); duplicate.push("a".to_string());
    let mut blank = Vec::new(); blank.push("".to_string());
    let mut selected = Vec::new(); selected.push("bb".to_string());
    let mut missing = Vec::new(); missing.push("ccc".to_string());
    proof {
        assert forall|i: int| 0 <= i < names@.len() implies names@[i]@ != ""@ by {
            assert(i == 0 || i == 1);
            assert(names@[i]@.len() == 1 || names@[i]@.len() == 2);
        }
        assert forall|i: int, j: int| 0 <= j < i < names@.len() implies names@[i]@ != names@[j]@ by {
            assert(i == 1 && j == 0);
            assert(names@[i]@.len() != names@[j]@.len());
        }
        assert(policy::unique_names(empty@) && policy::unique_names(names@));
        assert(duplicate@[0]@ == duplicate@[1]@ && duplicate@.len() == 2);
        assert(!policy::unique_names(duplicate@));
        assert(blank@[0]@ == ""@ && blank@.len() == 1);
        assert(!policy::unique_names(blank@));
        assert(selected@.len() == 1 && selected@[0]@ == names@[1]@);
        assert(policy::name_member(names@, selected@[0]@));
        assert forall|i: int| 0 <= i < names@.len() implies names@[i]@ != missing@[0]@ by {
            assert(i == 0 || i == 1);
            assert(names@[i]@.len() != missing@[0]@.len());
        }
        assert(!policy::name_member(names@, missing@[0]@));
        assert(!policy::name_member(empty@, selected@[0]@));
    }
    let empty_names = policy::valid_names(&empty);
    let valid = policy::valid_names(&names);
    let repeated = policy::valid_names(&duplicate);
    let blank_name = policy::valid_names(&blank);
    let empty_subset = policy::names_subset(&empty, &names);
    let selected_subset = policy::names_subset(&selected, &names);
    let missing_subset = policy::names_subset(&missing, &names);
    let no_available = policy::names_subset(&selected, &empty);
    expect(empty_names && valid && !repeated && !blank_name);
    expect(empty_subset && selected_subset && !missing_subset && !no_available);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn profile_modes_require_registry_bound_current_schema_and_one_owner_phase() {
    proof {
        reveal_strlit("chrono-bootstrap/v1"); reveal_strlit("unknown");
        reveal_strlit("git-main-worktree"); reveal_strlit("other");
        reveal_strlit("cargo"); reveal_strlit("rustc"); reveal_strlit("custom");
        assert("chrono-bootstrap/v1"@.len() != "unknown"@.len());
        assert("git-main-worktree"@.len() != "other"@.len());
        assert("cargo"@.len() != "custom"@.len() && "rustc"@.len() != "custom"@.len());
    }
    let ordinary = policy::profile_options("chrono-bootstrap/v1", true, false, false, true);
    let upgrade = policy::profile_options("chrono-bootstrap/v1", true, true, false, false);
    let stage = policy::profile_options("chrono-bootstrap/v1", true, false, true, true);
    let unknown = policy::profile_options("unknown", true, false, false, true);
    let unbound = policy::profile_options("chrono-bootstrap/v1", false, false, false, true);
    let conflict = policy::profile_options("chrono-bootstrap/v1", true, true, true, true);
    let no_upgrade = policy::profile_options("chrono-bootstrap/v1", true, false, false, false);
    let main = policy::participation_mode("git-main-worktree", true);
    let relative_git = policy::participation_mode("git-main-worktree", false);
    let other = policy::participation_mode("other", true);
    let zero = policy::unique_method(0); let one = policy::unique_method(1); let two = policy::unique_method(2);
    let cargo = policy::needs_configured_tool(false, "cargo");
    let compiler = policy::needs_configured_tool(false, "rustc");
    let custom = policy::needs_configured_tool(false, "custom");
    let prerequisite = policy::needs_configured_tool(true, "cargo");
    expect(ordinary && upgrade && stage && !unknown && !unbound && !conflict && !no_upgrade);
    expect(main && !relative_git && !other && !zero && one && !two);
    expect(!cargo && !compiler && custom && prerequisite);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn bootstrap_keeps_local_owners_and_uses_coordinator_only_to_seed_linked_hosts() {
    let initial_main = policy::bootstrap_uses_local_owner(true, false);
    let installed_main = policy::bootstrap_uses_local_owner(true, true);
    let initial_linked = policy::bootstrap_uses_local_owner(false, false);
    let installed_linked = policy::bootstrap_uses_local_owner(false, true);
    expect(initial_main && installed_main && !initial_linked && installed_linked);
}

fn source_observation(commit: &str, tree: &str, dirty: bool, error: Option<String>) -> (out: policy::SourceObservation)
    ensures
        out.commit is Some, out.commit.unwrap()@ == commit@,
        out.tree is Some, out.tree.unwrap()@ == tree@,
        out.dirty == Some(dirty), out.error == error,
{
    policy::SourceObservation {
        commit: Some(commit.to_string()), tree: Some(tree.to_string()), dirty: Some(dirty), error,
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn source_identity_only_exposes_commit_and_tree_for_unchanged_clean_observations() {
    proof { reveal_strlit("c"); reveal_strlit("cc"); reveal_strlit("t"); reveal_strlit("tt"); }
    let clean = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", false, None));
    let dirty = policy::source_identity(source_observation("c", "t", true, None), source_observation("c", "t", true, None));
    let commit = policy::source_identity(source_observation("c", "t", false, None), source_observation("cc", "t", false, None));
    let tree = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "tt", false, None));
    let status = policy::source_identity(source_observation("c", "t", false, None), source_observation("c", "t", true, None));
    let failed_before = policy::source_identity(source_observation("c", "t", true, Some("failed".to_string())), source_observation("cc", "tt", false, None));
    let failed_after = policy::source_identity(source_observation("c", "t", false, None), source_observation("cc", "tt", true, Some("failed".to_string())));
    expect(matches!(clean.state, policy::SourceState::Clean) && clean.commit.is_some() && clean.tree.is_some());
    expect(policy::artifact_identity::equal_text(clean.commit.unwrap().as_str(), "c") && policy::artifact_identity::equal_text(clean.tree.unwrap().as_str(), "t"));
    expect(matches!(dirty.state, policy::SourceState::Dirty) && dirty.commit.is_none() && dirty.tree.is_none());
    proof {
        assert(commit.before.commit.unwrap()@.len() == 1);
        assert(commit.after.commit.unwrap()@.len() == 2);
        assert(!policy::source_observations_same(&commit.before, &commit.after));
        assert(tree.before.tree.unwrap()@.len() == 1);
        assert(tree.after.tree.unwrap()@.len() == 2);
        assert(!policy::source_observations_same(&tree.before, &tree.after));
    }
    expect(matches!(commit.state, policy::SourceState::Changed) && commit.commit.is_none() && commit.tree.is_none());
    expect(matches!(tree.state, policy::SourceState::Changed) && tree.commit.is_none() && tree.tree.is_none());
    expect(matches!(status.state, policy::SourceState::Changed) && status.commit.is_none() && status.tree.is_none());
    expect(matches!(failed_before.state, policy::SourceState::Unavailable) && failed_before.commit.is_none() && failed_before.tree.is_none());
    expect(matches!(failed_after.state, policy::SourceState::Unavailable) && failed_after.commit.is_none() && failed_after.tree.is_none());
    expect(failed_before.before.error.is_some() && failed_after.after.error.is_some());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn exit_status_keeps_original_codes_and_only_negates_positive_signals() {
    let success = policy::process_exit(Some(0), None);
    let failure = policy::process_exit(Some(17), None);
    let precedence = policy::process_exit(Some(17), Some(9));
    let signal = policy::process_exit(None, Some(9));
    let missing = policy::process_exit(None, None);
    let zero_signal = policy::process_exit(None, Some(0));
    let negative_signal = policy::process_exit(None, Some(-1));
    let minimum_code = policy::process_exit(Some(-2147483648i32), Some(9));
    let maximum_code = policy::process_exit(Some(2147483647i32), None);
    let maximum_signal = policy::process_exit(None, Some(2147483647i32));
    let minimum_signal = policy::process_exit(None, Some(-2147483648i32));
    expect(matches!(success, Some(0i32)) && matches!(failure, Some(17i32)) && matches!(precedence, Some(17i32)));
    expect(matches!(signal, Some(-9i32)) && missing.is_none());
    expect(zero_signal.is_none() && negative_signal.is_none() && minimum_signal.is_none());
    expect(minimum_code.is_some() && minimum_code.unwrap() == i32::MIN && matches!(maximum_code, Some(2147483647i32)));
    expect(matches!(maximum_signal, Some(-2147483647i32)));
}

fn executable(path: &str, hash: &str, version: &str) -> (out: Executable)
    ensures out.path@ == path@, out.sha256@ == hash@, out.version@ == version@,
{
    Executable { path: path.to_string(), sha256: hash.to_string(), version: version.to_string() }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn environment_names_reject_only_the_reserved_prefix_equals_and_nul() {
    let empty: [u8; 0] = [];
    let normal = policy::environment_key(&[65u8]);
    let missing = policy::environment_key(&empty);
    let equals = policy::environment_key(&[65u8, 61]);
    let nul = policy::environment_key(&[65u8, 0]);
    let reserved = policy::environment_key(&[67u8, 72, 82, 79, 78, 79, 95]);
    let shorter = policy::environment_key(&[67u8, 72, 82, 79, 78, 79]);
    let value_empty = policy::environment_value(&empty);
    let value_lines = policy::environment_value(&[10u8, 13, 61]);
    let value_nul = policy::environment_value(&[10u8, 0]);
    expect(normal && !missing && !equals && !nul && !reserved && shorter);
    expect(value_empty && value_lines && !value_nul);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn host_paths_require_the_declared_prefix_and_exact_normal_components() {
    let prefix = [97u8, 47];
    let good_bytes = [97u8, 47, 98];
    let hidden_bytes = [97u8, 47, 46, 98];
    let repeated_bytes = [97u8, 47, 47, 98];
    let current_bytes = [97u8, 47, 46, 47, 98];
    let parent_bytes = [97u8, 47, 46, 46];
    let trailing_bytes = [97u8, 47, 98, 47];
    let nul_bytes = [97u8, 47, 0];
    proof {
        assert forall|i: int| 0 <= i < good_bytes@.len() implies !policy::bad_path_byte(good_bytes@, i) by {
            assert(i == 0 || i == 1 || i == 2);
        }
        assert forall|i: int| 0 <= i < hidden_bytes@.len() implies !policy::bad_path_byte(hidden_bytes@, i) by {
            assert(i == 0 || i == 1 || i == 2 || i == 3);
        }
        assert(policy::bad_path_byte(repeated_bytes@, 2));
        assert(policy::bad_path_byte(current_bytes@, 2));
        assert(policy::bad_path_byte(parent_bytes@, 2));
        assert(policy::bad_path_byte(trailing_bytes@, 3));
        assert(policy::bad_path_byte(nul_bytes@, 2));
    }
    let good = policy::host_path(&good_bytes, &prefix);
    let hidden = policy::host_path(&hidden_bytes, &prefix);
    let repeated = policy::host_path(&repeated_bytes, &prefix);
    let current = policy::host_path(&current_bytes, &prefix);
    let parent = policy::host_path(&parent_bytes, &prefix);
    let trailing = policy::host_path(&trailing_bytes, &prefix);
    let nul = policy::host_path(&nul_bytes, &prefix);
    let wrong_prefix = policy::host_path(&[98u8, 47, 97], &prefix);
    expect(good && hidden && !repeated && !current && !parent && !trailing && !nul && !wrong_prefix);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn host_paths_compare_full_components_and_host_variables_are_exact() {
    proof {
        reveal_strlit("HOME"); reveal_strlit("TMPDIR"); reveal_strlit("CARGO_HOME"); reveal_strlit("RUSTUP_HOME");
        assert("RUSTUP_HOME"@.len() != "HOME"@.len());
        assert("RUSTUP_HOME"@.len() != "TMPDIR"@.len());
        assert("RUSTUP_HOME"@.len() != "CARGO_HOME"@.len());
    }
    let separate = policy::disjoint_homes(&[97u8], &[98u8]);
    let neighbor = policy::disjoint_homes(&[97u8], &[97u8, 98]);
    let same = policy::disjoint_homes(&[97u8], &[97u8]);
    let child = policy::disjoint_homes(&[97u8], &[97u8, 47, 98]);
    let parent = policy::disjoint_homes(&[97u8, 47, 98], &[97u8]);
    let home = policy::host_variable("HOME");
    let cargo = policy::host_variable("CARGO_HOME");
    let temp = policy::host_variable("TMPDIR");
    let rustup = policy::host_variable("RUSTUP_HOME");
    expect(separate && neighbor && !same && !child && !parent);
    expect(home && cargo && temp && !rustup);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn toolchain_probes_choose_only_the_required_repairs() {
    let ordinary_cargo = policy::probe_action(false, true);
    let ordinary_format = policy::probe_action(false, false);
    let ready_cargo = policy::probe_action(true, true);
    let ready_format = policy::probe_action(true, false);
    expect(matches!(ordinary_cargo, policy::ProbeAction::InstallMinimal) && matches!(ordinary_format, policy::ProbeAction::AddFormat));
    expect(matches!(ready_cargo, policy::ProbeAction::Ready) && matches!(ready_format, policy::ProbeAction::Ready));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn native_binding_requires_both_fields_and_an_explicit_platform() {
    proof {
        reveal_strlit("Darwin"); reveal_strlit("Linux");
        reveal_strlit("arm64"); reveal_strlit("x86_64");
        assert("Darwin"@.len() != "Linux"@.len());
        assert("arm64"@.len() != "x86_64"@.len());
    }
    let native = policy::NativePlatform { system: "Darwin".to_string(), machine: "arm64".to_string() };
    let system = policy::NativePlatform { system: "Linux".to_string(), machine: "arm64".to_string() };
    let machine = policy::NativePlatform { system: "Darwin".to_string(), machine: "x86_64".to_string() };
    let exact = policy::platform_matches(Some(&native), &native);
    let absent = policy::platform_matches(None, &native);
    let wrong_system = policy::platform_matches(Some(&system), &native);
    let wrong_machine = policy::platform_matches(Some(&machine), &native);
    expect(exact && !absent && !wrong_system && !wrong_machine);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_identity_binds_schema_and_toolchain_independently() {
    proof {
        reveal_strlit("chrono-bootstrap-toolchains/v1");
        reveal_strlit("chrono-bootstrap-toolchains/v2");
        reveal_strlit("1.95.0"); reveal_strlit("1.95.0x");
        assert("chrono-bootstrap-toolchains/v1"@[29] != "chrono-bootstrap-toolchains/v2"@[29]);
        assert("1.95.0"@.len() != "1.95.0x"@.len());
    }
    let exact = policy::registry_matches("chrono-bootstrap-toolchains/v1", "1.95.0", "1.95.0");
    let schema = policy::registry_matches("chrono-bootstrap-toolchains/v2", "1.95.0", "1.95.0");
    let version = policy::registry_matches("chrono-bootstrap-toolchains/v1", "1.95.0x", "1.95.0");
    expect(exact && !schema && !version);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn version_prefix_requires_a_delimiter_and_only_cargo_allows_an_exact_version() {
    let prefix = [97u8];
    let exact = [97u8];
    let spaced = [97u8, 32, 98];
    let glued = [97u8, 98];
    let wrong = [98u8, 32];
    let cargo = policy::version_matches(&exact, &prefix, true);
    let rustc = policy::version_matches(&exact, &prefix, false);
    let with_suffix = policy::version_matches(&spaced, &prefix, false);
    let without_delimiter = policy::version_matches(&glued, &prefix, true);
    let wrong_prefix = policy::version_matches(&wrong, &prefix, true);
    let short = policy::version_matches(&prefix, &spaced, true);
    expect(cargo && !rustc && with_suffix && !without_delimiter && !wrong_prefix && !short);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registry_rejects_direct_or_native_overrides_even_when_bindings_are_optional() {
    let normal = policy::declaration_source(true, false, false, true, false);
    let direct = policy::declaration_source(true, true, false, false, true);
    let native = policy::declaration_source(true, false, true, false, true);
    let both = policy::declaration_source(true, true, true, true, true);
    expect(matches!(normal, policy::DeclarationSource::Registry));
    expect(matches!(direct, policy::DeclarationSource::Reject) && matches!(native, policy::DeclarationSource::Reject) && matches!(both, policy::DeclarationSource::Reject));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn required_direct_bindings_need_a_matching_platform_and_declaration() {
    let accepted = policy::declaration_source(false, true, true, true, true);
    let wrong = policy::declaration_source(false, true, true, true, false);
    let missing = policy::declaration_source(false, false, true, true, true);
    let missing_platform = policy::declaration_source(false, true, false, true, true);
    let optional = policy::declaration_source(false, true, false, false, false);
    let unbound = policy::declaration_source(false, false, false, false, false);
    expect(matches!(accepted, policy::DeclarationSource::Direct) && matches!(optional, policy::DeclarationSource::Direct));
    expect(matches!(wrong, policy::DeclarationSource::Reject) && matches!(missing, policy::DeclarationSource::Reject) && matches!(missing_platform, policy::DeclarationSource::Reject));
    expect(matches!(unbound, policy::DeclarationSource::Unbound));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn relocation_keeps_digest_and_version_and_only_ignores_the_old_path() {
    proof {
        reveal_strlit("old"); reveal_strlit("new"); reveal_strlit("hash");
        reveal_strlit("other"); reveal_strlit("version");
        assert("old"@[0] != "new"@[0]);
        assert("hash"@.len() != "other"@.len());
        assert("version"@.len() != "other"@.len());
    }
    let expected = executable("old", "hash", "version");
    let relocated = executable("new", "hash", "version");
    let accepted = policy::selected_matches(&relocated, &expected, true);
    let exact = policy::selected_matches(&relocated, &expected, false);
    let hash = executable("new", "other", "version");
    let version = executable("new", "hash", "other");
    let wrong_hash = policy::selected_matches(&hash, &expected, true);
    let wrong_version = policy::selected_matches(&version, &expected, true);
    expect(accepted && !exact && !wrong_hash && !wrong_version);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn only_the_repeated_relocatable_declared_path_is_retired() {
    proof {
        reveal_strlit("old"); reveal_strlit("current");
        assert("old"@.len() != "current"@.len());
    }
    let retired = policy::configured_action(Some("old"), Some("old"), true, false);
    let exact = policy::configured_action(Some("old"), Some("old"), false, false);
    let current = policy::configured_action(Some("current"), Some("old"), true, false);
    let missing = policy::configured_action(None, Some("old"), true, false);
    let registry = policy::configured_action(None, None, true, true);
    let configured_registry = policy::configured_action(Some("current"), None, true, true);
    expect(matches!(retired, policy::ConfiguredAction::Retired));
    expect(matches!(exact, policy::ConfiguredAction::Observe) && matches!(current, policy::ConfiguredAction::Observe) && matches!(configured_registry, policy::ConfiguredAction::Observe));
    expect(matches!(missing, policy::ConfiguredAction::Missing) && matches!(registry, policy::ConfiguredAction::NotDeclared));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inherited_compiler_and_both_wrapper_slots_are_independent_constraints() {
    proof {
        reveal_strlit("selected"); reveal_strlit("other"); reveal_strlit("");
        assert("selected"@.len() != "other"@.len());
        assert("other"@.len() != 0);
    }
    let absent = policy::compiler_selection("selected", None, None, None);
    let empty = policy::compiler_selection("selected", Some(""), Some(""), Some(""));
    let exact = policy::compiler_selection("selected", Some("selected"), None, None);
    let wrong = policy::compiler_selection("selected", Some("other"), None, None);
    let wrapper = policy::compiler_selection("selected", None, Some("other"), None);
    let workspace = policy::compiler_selection("selected", None, None, Some("other"));
    expect(absent && empty && exact && !wrong && !wrapper && !workspace);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn sha256_allows_zero_but_requires_exact_lowercase_hex_length() {
    broadcast use vstd::array::group_array_axioms;
    let zeros = [48u8; 64];
    let short = [48u8; 63];
    let long = [48u8; 65];
    let mut upper = [48u8; 64];
    let mut nul = [48u8; 64];
    upper[63] = 65;
    nul[0] = 0;
    proof {
        assert(zeros@.len() == 64);
        assert forall|i: int| 0 <= i < zeros@.len()
            implies policy::hash_byte(zeros@[i]) by {
            assert(zeros@[i] == 48);
        }
        assert(short@.len() == 63);
        assert(long@.len() == 65);
        assert(upper@[63] == 65);
        assert(nul@[0] == 0);
    }
    let accepted = policy::valid_sha256(&zeros);
    let short_hash = policy::valid_sha256(&short);
    let long_hash = policy::valid_sha256(&long);
    let upper_hash = policy::valid_sha256(&upper);
    let nul_hash = policy::valid_sha256(&nul);
    let direct = policy::binding_valid(&zeros, false, true);
    let missing = policy::binding_valid(&zeros, false, false);
    let registry = policy::binding_valid(&zeros, true, false);
    let invalid_registry = policy::binding_valid(&upper, true, true);
    expect(accepted);
    expect(!short_hash);
    expect(!long_hash);
    expect(!upper_hash);
    expect(!nul_hash);
    expect(direct && !missing && registry && !invalid_registry);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn template_repository_checks_observed_ascii_owner_and_name_before_writing() {
    broadcast use vstd::array::group_array_axioms;
    let mut path = [65u8, 45, 46, 95, 49];
    let host = "github.com".as_bytes();
    let ssh = "ssh.github.com".as_bytes();
    let name = [120u8];
    let dot = [46u8];
    let dotdot = [46u8,46];
    let outside = [255u8];
    proof {
        assert forall|i:int| 0 <= i < path.len() implies policy::github_slug_byte(path@[i]) by {
            assert(i == 0 || i == 1 || i == 2 || i == 3 || i == 4);
        }
        assert(policy::github_slug(path@));
        assert(policy::github_slug(name@));
        assert(dot@ =~= seq![46u8]);
        assert(dotdot@ =~= seq![46u8,46u8]);
        assert(!policy::github_slug_byte(outside@[0]));
    }
    expect(policy::github_repository(host,&path,&name));
    expect(policy::github_repository(ssh,&name,&path));
    expect(!policy::github_repository(host,&[],&name));
    expect(!policy::github_repository(host,&dot,&name));
    expect(!policy::github_repository(host,&name,&dotdot));
    path[3] = 47;
    assert(!policy::github_slug_byte(path@[3]));
    expect(!policy::github_repository(host,&path,&name));
    expect(!policy::github_repository(host,&name,&outside));
    proof {
        reveal_strlit("github.com"); reveal_strlit("ssh.github.com");
    }
    expect(!policy::github_repository(&[],&name,&name));
}

}

#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::environment as admission;
#[cfg(verus_keep_ghost)]
use policy::environment as admission;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn host_directory_admission_stops_at_the_first_unusable_entry() {
    use admission::{HostSelectionError, host_paths};
    let ok = |_value: &String| -> (result: Result<u64, u64>)
        ensures result == Ok::<u64, u64>(17),
    { Ok(17u64) };
    let bad = |_value: &String| -> (result: Result<u64, u64>)
        ensures result == Err::<u64, u64>(41),
    { Err(41u64) };
    let absent = host_paths(None, ok);
    expect(match absent { Ok(paths) => paths.len() == 0, _ => false });
    let empty: Vec<(String, String, bool)> = Vec::new();
    let present_empty = host_paths(Some(empty.as_slice()), ok);
    expect(matches!(present_empty, Err(HostSelectionError::Empty)));
    let mut rows = Vec::new();
    rows.push(("HOME".to_string(), "a".to_string(), false));
    let accepted = host_paths(Some(rows.as_slice()), ok);
    match accepted {
        Ok(paths) => {
            expect(paths.len() == 1);
            assert(admission::resolved(rows[0], paths[0], ok));
            expect(paths[0] == 17);
        },
        _ => expect(false),
    }
    rows.push(("CARGO_HOME".to_string(), "b".to_string(), true));
    let conflict = host_paths(Some(rows.as_slice()), ok);
    match conflict {
        Err(HostSelectionError::Key(_)) => {},
        Ok(_paths) => {
            assert(admission::resolved(rows[1], _paths[1], ok));
            expect(false);
        },
        Err(HostSelectionError::Path(_index, _)) => {
            proof { assert(_index@ == 0 || _index@ == 1); }
            expect(false);
        },
        _ => expect(false),
    }
    let path = host_paths(Some(rows.as_slice()), bad);
    match path {
        Err(HostSelectionError::Path(_, value)) => expect(value == 41),
        Err(HostSelectionError::Key(_index)) => {
            proof {
                assert(_index@ == 1);
                assert(admission::could_resolve(rows[0], bad));
                let p = choose|p: u64| admission::resolved(rows[0], p, bad);
            }
            expect(false);
        },
        Ok(_paths) => {
            assert(admission::resolved(rows[0], _paths[0], bad));
            expect(false);
        },
        _ => expect(false),
    }
}
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn prepared_environment_selects_all_paths_before_creating_any_directory() {
    use chrono_host_bootstrap::bindings::Environment;
    use chrono_host_bootstrap::environment::Prepared;
    use serde_json::{Value, json};
    use std::ffi::OsStr;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut inherited = Environment::new();
    inherited.insert("HOME".into(), "/inherited".into());
    inherited.insert("KEEP".into(), "retained".into());
    let config = |explicit: Value, host: Option<Value>| {
        let mut data = json!({"rust_toolchain":"fixture","environment":explicit});
        if let Some(host) = host {
            data["host_environment"] = host;
        }
        serde_json::to_vec(&data).unwrap()
    };
    let absent = Prepared::parse(root, &config(json!({}), None), &inherited).unwrap();
    assert!(absent.host_directories.is_empty());
    assert_eq!(absent.environment, inherited);
    let dirs = [
        ".chrono-harness/cache/cargo",
        ".chrono-harness/cache/home",
        ".chrono-harness/cache/tmp",
    ];
    let chosen = Prepared::parse(
        root,
        &config(
            json!({"EXPLICIT":"one"}),
            Some(json!({
                "HOME":dirs[1], "CARGO_HOME":dirs[0], "TMPDIR":dirs[2],
            })),
        ),
        &inherited,
    )
    .unwrap();
    assert_eq!(chosen.host_directories, dirs.map(|p| root.join(p)));
    for (key, relative) in ["CARGO_HOME", "HOME", "TMPDIR"].into_iter().zip(dirs) {
        assert_eq!(
            chosen.environment[OsStr::new(key)],
            root.join(relative).as_os_str()
        );
    }
    assert_eq!(chosen.environment[OsStr::new("EXPLICIT")], "one");
    assert_eq!(chosen.environment[OsStr::new("KEEP")], "retained");
    for (explicit, host, message) in [
        (
            json!({}),
            json!({}),
            "empty bootstrap host environment selection",
        ),
        (
            json!({"HOME":"explicit"}),
            json!({"CARGO_HOME":dirs[0],"HOME":dirs[1]}),
            "invalid or conflicting bootstrap host environment selection",
        ),
        (
            json!({}),
            json!({"CARGO_HOME":dirs[0],"UNSUPPORTED":dirs[1]}),
            "invalid or conflicting bootstrap host environment selection",
        ),
        (
            json!({}),
            json!({"CARGO_HOME":"outside","UNSUPPORTED":dirs[1]}),
            "invalid bootstrap path: outside",
        ),
        (
            json!({"CHRONO_RESERVED":"x"}),
            json!({}),
            "invalid bootstrap environment selection",
        ),
        (
            json!({"OK":"x\u{0}y"}),
            json!({}),
            "invalid bootstrap environment selection",
        ),
    ] {
        let error = Prepared::parse(root, &config(explicit, Some(host)), &inherited)
            .err()
            .unwrap();
        assert_eq!(error, message);
        assert!(!root.join(".chrono-harness").exists());
    }
    assert_eq!(inherited[OsStr::new("HOME")], "/inherited");
}
