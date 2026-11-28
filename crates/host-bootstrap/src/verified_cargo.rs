//! 官方 Cargo-Verus 原输出的顺序合同；JSON 和文件观察由调用方传入。
#[cfg(verus_keep_ghost)]
use super::artifact_identity;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::artifact_identity;
use artifact_identity::{equal_bytes, equal_text};
#[cfg(verus_keep_ghost)]
#[path = "proved_build.rs"]
pub mod proved_build;
#[cfg(not(verus_keep_ghost))]
pub use crate::proved_build::{
    NativeHeader, OwnerSourceMode, native_header, owner_source_mode, required_compiler_paths,
};
#[cfg(verus_keep_ghost)]
pub use proved_build::{
    NativeHeader, OwnerSourceMode, compiler_has_path, native_header, owner_source_mode,
    required_compiler_paths,
};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
use vstd::slice::{slice_subrange, slice_to_vec};
#[cfg(not(verus_keep_ghost))]
use vstd::{pervasive::VecAdditionalExecFns, prelude::verus};

verus! {

/// The decoder validates each compiler report against the selected output contract.
pub enum Record { Proof(bool), Cargo(Cargo) }

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
#[cfg_attr(not(verus_keep_ghost), serde(tag = "reason"))]
pub enum Cargo {
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "compiler-artifact"))]
    Artifact(Artifact),
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "compiler-message"))]
    Message { message: Message },
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "build-script-executed"))]
    Script { package_id: String, linked_libs: Vec<String>, linked_paths: Vec<String>,
        cfgs: Vec<String>, env: Vec<(String, String)>, out_dir: String },
    #[cfg_attr(not(verus_keep_ghost), serde(rename = "build-finished"))]
    Finished { success: bool },
}

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
pub struct Message { pub level: String }

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Target {
    pub name: String, pub src_path: String, pub kind: Vec<String>, pub crate_types: Vec<String>,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Artifact {
    pub package_id: String,
    pub manifest_path: String,
    pub target: Target,
    pub filenames: Vec<String>,
    pub executable: Option<String>,
    pub fresh: bool,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, serde::Deserialize, serde::Serialize))]
pub struct Identity { pub path: String, pub sha256: String, pub length: u64 }

/// The fields reported by Cargo that the native coverage check consumes.
#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
pub struct NativeRow {
    pub reason: String,
    pub manifest_path: Option<String>,
    pub target: Option<NativeTarget>,
    pub filenames: Option<Vec<String>>,
}

#[cfg_attr(not(verus_keep_ghost), derive(serde::Deserialize))]
pub struct NativeTarget {
    pub kind: Vec<String>,
}

pub open spec fn native_artifact(row: &NativeRow, manifest: Seq<char>) -> bool {
    row.reason@ == "compiler-artifact"@
        && match row.manifest_path { Some(path) => path@ == manifest, None => false }
        && match row.target {
            Some(target) => target.kind.len() == 1 && target.kind[0]@ == "lib"@,
            None => false,
        }
}

fn native_artifact_matches(row: &NativeRow, manifest: &str) -> (matches: bool)
    ensures matches == native_artifact(row, manifest@),
{
    equal_text(row.reason.as_str(), "compiler-artifact")
        && match &row.manifest_path {
            Some(path) => equal_text(path.as_str(), manifest),
            None => false,
        }
        && match &row.target {
            Some(target) => target.kind.len() == 1 && equal_text(target.kind[0].as_str(), "lib"),
            None => false,
        }
}

pub open spec fn is_metadata(path: Seq<u8>) -> bool {
    path.len() >= 6 && path.skip(path.len() - 6) == seq![46u8, 114, 109, 101, 116, 97]
}

fn metadata_suffix(path: &[u8]) -> (matches: bool)
    ensures matches == is_metadata(path@),
{
    path.len() >= 6 && equal_bytes(slice_subrange(path, path.len() - 6, path.len()), &[46u8, 114, 109, 101, 116, 97])
}

pub open spec fn metadata_files(files: Seq<String>) -> Seq<String>
    decreases files.len(),
{
    if files.len() == 0 { Seq::empty() }
    else {
        let last = files.last();
        let previous = metadata_files(files.take(files.len() - 1));
        if is_metadata(vstd::utf8::encode_utf8(last@)) { previous.push(last) }
        else { previous }
    }
}

spec fn all_metadata(files: Seq<String>) -> bool {
    forall|i: int| 0 <= i < files.len() ==>
        is_metadata(vstd::utf8::encode_utf8(#[trigger] files[i]@))
}

fn append_metadata(files: &Vec<String>, found: &mut Vec<String>)
    requires all_metadata(old(found)@),
    ensures found@ == old(found)@ + metadata_files(files@), all_metadata(found@),
{
    let mut i = 0;
    while i < files.len()
        invariant 0 <= i <= files.len(),
            found@ == old(found)@ + metadata_files(files@.take(i as int)),
            all_metadata(found@),
        decreases files.len() - i,
    {
        let file = &files[i];
        let ghost before = found@;
        if metadata_suffix(file.as_str().as_bytes()) {
            found.push(file.clone());
            assert(found@ =~= before.push(*file));
            assert forall|j: int| 0 <= j < found.len() implies
                is_metadata(vstd::utf8::encode_utf8(#[trigger] found@[j]@)) by {
                if j < before.len() { assert(found@[j] == before[j]); }
                else { assert(found@[j] == *file); }
            }
        }
        proof {
            let next = files@.take(i + 1);
            assert(next.last() == *file);
            assert(next.take(i as int) =~= files@.take(i as int));
            assert(found@ =~= old(found)@ + metadata_files(next));
        }
        i += 1;
    }
    assert(files@.take(files.len() as int) =~= files@);
}

pub open spec fn cargo_metadata(rows: Seq<NativeRow>, manifest: Seq<char>) -> Seq<String>
    decreases rows.len(),
{
    if rows.len() == 0 { Seq::empty() }
    else {
        let row = rows.last();
        let previous = cargo_metadata(rows.take(rows.len() - 1), manifest);
        if native_artifact(&row, manifest) {
            match row.filenames { Some(files) => previous + metadata_files(files@), None => previous }
        } else { previous }
    }
}

pub open spec fn missing_native_filenames(rows: Seq<NativeRow>, manifest: Seq<char>) -> bool {
    exists|i: int| 0 <= i < rows.len()
        && native_artifact(&#[trigger] rows[i], manifest) && rows[i].filenames.is_none()
}

pub enum NativeLibrary { InvalidFilenames, Missing, Unique(String), Ambiguous }

pub fn select_native_library(rows: &Vec<NativeRow>, manifest: &str) -> (selection: NativeLibrary)
    ensures match selection {
        NativeLibrary::InvalidFilenames => missing_native_filenames(rows@, manifest@),
        NativeLibrary::Missing => !missing_native_filenames(rows@, manifest@)
            && cargo_metadata(rows@, manifest@).len() == 0,
        NativeLibrary::Unique(path) => !missing_native_filenames(rows@, manifest@)
            && cargo_metadata(rows@, manifest@) == seq![path] && is_metadata(vstd::utf8::encode_utf8(path@)),
        NativeLibrary::Ambiguous => !missing_native_filenames(rows@, manifest@)
            && cargo_metadata(rows@, manifest@).len() > 1,
    },
{
    let mut found = Vec::new();
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(),
            found@ == cargo_metadata(rows@.take(i as int), manifest@),
            all_metadata(found@),
            forall|j: int| 0 <= j < i && native_artifact(&#[trigger] rows@[j], manifest@)
                ==> rows@[j].filenames.is_some(),
        decreases rows.len() - i,
    {
        let row = &rows[i];
        if native_artifact_matches(row, manifest) {
            let Some(files) = &row.filenames else {
                assert(rows@[i as int].filenames.is_none());
                return NativeLibrary::InvalidFilenames;
            };
            append_metadata(files, &mut found);
        }
        proof {
            let next = rows@.take(i + 1);
            assert(next.last() == *row);
            assert(next.take(i as int) =~= rows@.take(i as int));
            assert(found@ == cargo_metadata(next, manifest@));
        }
        i += 1;
    }
    assert(rows@.take(rows.len() as int) =~= rows@);
    if found.len() == 0 { NativeLibrary::Missing }
    else if found.len() == 1 {
        assert(found@ =~= seq![found@[0]]);
        NativeLibrary::Unique(found[0].clone())
    } else { NativeLibrary::Ambiguous }
}

pub fn native_library_arguments<'a>(args: &'a [String]) -> (selection: Option<(&'a str, &'a str)>)
    ensures selection.is_some() == (args.len() == 3 && args@[0]@ == "select-native-cargo-library"@),
        selection.is_some() ==> selection.unwrap().0@ == args@[1]@ && selection.unwrap().1@ == args@[2]@,
{
    if args.len() != 3 || !equal_text(args[0].as_str(), "select-native-cargo-library") { return None; }
    Some((args[1].as_str(), args[2].as_str()))
}

pub open spec fn basename_start(path: Seq<u8>, start: int) -> bool {
    0 <= start <= path.len() && (start == 0 || path[start - 1] == 47)
        && forall|i: int| start <= i < path.len() ==> path[i] != 47
}

pub open spec fn lib_prefix(path: Seq<u8>, start: int) -> bool {
    start + 3 <= path.len() && path.subrange(start, start + 3) == seq![108u8, 105, 98]
}

pub open spec fn stem_at(path: Seq<u8>, start: int) -> Seq<u8> {
    path.take(start) + path.subrange(if lib_prefix(path, start) { start + 3 } else { start }, path.len() as int)
}

pub open spec fn dep_info_stem(path: Seq<u8>) -> Seq<u8> {
    let name = path.take(path.len() - 6);
    let start = choose|i: int| basename_start(name, i);
    stem_at(name, start)
}

pub proof fn stem_at_basename(path: Seq<u8>, start: int)
    requires basename_start(path.take(path.len() - 6), start),
    ensures dep_info_stem(path) == stem_at(path.take(path.len() - 6), start),
{
    let name = path.take(path.len() - 6);
    assert(exists|i: int| #[trigger] basename_start(name, i));
    let chosen = choose|i: int| basename_start(name, i);
    if chosen < start { assert(name[start - 1] != 47); }
    if start < chosen { assert(name[chosen - 1] != 47); }
}

/// Convert Cargo's selected metadata filename to the existing native query's stem.
pub fn native_dep_info_stem(path: &[u8]) -> (stem: Vec<u8>)
    requires is_metadata(path@),
    ensures stem@ == dep_info_stem(path@),
{
    let end = path.len() - 6;
    let mut start = end;
    while start > 0 && path[start - 1] != 47
        invariant 0 <= start <= end, end == path.len() - 6,
            forall|i: int| start <= i < end ==> path@[i] != 47,
        decreases start,
    {
        start -= 1;
    }
    assert(basename_start(path@.take(end as int), start as int));
    let skip = end - start >= 3
        && equal_bytes(slice_subrange(path, start, start + 3), &[108u8, 105, 98]);
    assert(skip == lib_prefix(path@.take(end as int), start as int)) by {
        if end - start >= 3 {
            assert(path@.take(end as int).subrange(start as int, start + 3)
                =~= path@.subrange(start as int, start + 3));
        }
    }
    let mut stem = slice_to_vec(slice_subrange(path, 0, start));
    let rest = slice_subrange(path, if skip { start + 3 } else { start }, end);
    extend(&mut stem, rest);
    proof {
        stem_at_basename(path@, start as int);
        assert(path@.take(end as int).take(start as int) =~= path@.take(start as int));
        let offset = if skip { start as int + 3 } else { start as int };
        assert(path@.take(end as int).subrange(offset, end as int) =~= rest@);
        assert(stem@ =~= dep_info_stem(path@));
    }
    stem
}

pub fn bounded_original(regular: bool, bytes: u64) -> (valid: bool)
    ensures valid == (regular && bytes <= 8_388_608),
{ regular && bytes <= 8_388_608 }

pub fn content_matches(actual: &str, length: u64, expected: &str, expected_length: u64) -> (same: bool)
    ensures same == (actual@ == expected@ && length == expected_length),
{ equal_text(actual, expected) && length == expected_length }

pub open spec fn selected_target(
    artifact: &Artifact, manifest: Seq<char>, source: Seq<char>, name: Seq<char>, kind: Seq<char>,
) -> bool {
    artifact.manifest_path@ == manifest && artifact.target.src_path@ == source
        && artifact.target.name@ == name && artifact.target.kind.len() == 1
        && artifact.target.kind[0]@ == kind && artifact.target.crate_types.len() == 1
        && artifact.target.crate_types[0]@ == kind
}

fn target_fields_match(artifact: &Artifact, manifest: &str, source: &str, name: &str, kind: &str, crate_type: &str) -> (selected: bool)
    ensures selected == (artifact.manifest_path@ == manifest@ && artifact.target.src_path@ == source@
        && artifact.target.name@ == name@ && artifact.target.kind.len() == 1
        && artifact.target.kind[0]@ == kind@ && artifact.target.crate_types.len() == 1
        && artifact.target.crate_types[0]@ == crate_type@),
{
    equal_text(artifact.manifest_path.as_str(), manifest)
        && equal_text(artifact.target.src_path.as_str(), source)
        && equal_text(artifact.target.name.as_str(), name)
        && artifact.target.kind.len() == 1 && equal_text(artifact.target.kind[0].as_str(), kind)
        && artifact.target.crate_types.len() == 1 && equal_text(artifact.target.crate_types[0].as_str(), crate_type)
}

pub fn artifact_selected(artifact: &Artifact, manifest: &str, source: &str, name: &str, kind: &str) -> (selected: bool)
    ensures selected == selected_target(artifact, manifest@, source@, name@, kind@),
{ target_fields_match(artifact, manifest, source, name, kind, kind) }

pub fn library_selected(
    artifact: &Artifact, package_id: &str, manifest: &str, source: &str, name: &str, kind: &str,
) -> (selected: bool)
    ensures selected == (artifact.package_id@ == package_id@ && artifact.manifest_path@ == manifest@
        && artifact.target.src_path@ == source@ && artifact.target.name@ == name@
        && artifact.target.kind.len() == 1 && artifact.target.kind[0]@ == kind@
        && artifact.target.crate_types.len() == 1 && artifact.target.crate_types[0]@ == "lib"@),
{
    equal_text(artifact.package_id.as_str(), package_id)
        && target_fields_match(artifact, manifest, source, name, kind, "lib")
}

pub fn diagnostic_rejected(level: &str) -> (rejected: bool)
    ensures rejected == (level@ != "warning"@ && level@ != "note"@ && level@ != "help"@),
{ !equal_text(level, "warning") && !equal_text(level, "note") && !equal_text(level, "help") }

pub fn registered_sources(schema: &str, source: &str, sources: &Vec<String>) -> (valid: bool)
    ensures valid == (schema@ == "chrono-proved-build/v1"@ && sources.len() > 0
        && exists|i: int| 0 <= i < sources.len() && sources[i]@ == source@),
{
    if !equal_text(schema, "chrono-proved-build/v1") || sources.len() == 0 { return false; }
    let mut i = 0;
    while i < sources.len()
        invariant 0 <= i <= sources.len(), sources.len() > 0, schema@ == "chrono-proved-build/v1"@,
            forall|j: int| 0 <= j < i ==> sources[j]@ != source@,
        decreases sources.len() - i,
    {
        if equal_text(sources[i].as_str(), source) { return true; }
        i += 1;
    }
    false
}

pub open spec fn published_binary(artifact: &Artifact, path: Seq<char>) -> bool {
    artifact.filenames.len() == 1 && artifact.filenames[0]@ == path
        && match artifact.executable { Some(value) => value@ == path, None => false }
}

pub fn binary_shape(artifact: &Artifact, path: &str) -> (valid: bool)
    ensures valid == published_binary(artifact, path@),
{
    artifact.filenames.len() == 1 && equal_text(artifact.filenames[0].as_str(), path)
        && match &artifact.executable { Some(value) => equal_text(value.as_str(), path), None => false }
}

pub enum Recovery { Continue, Rebuild, Reject }

pub fn recovery(fresh: bool, valid: bool, available: bool) -> (action: Recovery)
    ensures match action {
        Recovery::Continue => !fresh || valid,
        Recovery::Rebuild => fresh && !valid && available,
        Recovery::Reject => fresh && !valid && !available,
    },
{
    if !fresh || valid { Recovery::Continue }
    else if available { Recovery::Rebuild } else { Recovery::Reject }
}

pub open spec fn text_lists_match(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len() && forall|i: int| 0 <= i < first.len() ==> first[i]@ == second[i]@
}

pub fn same_text_list(first: &Vec<String>, second: &Vec<String>) -> (same: bool)
    ensures same == text_lists_match(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len() == second.len(),
            forall|j: int| 0 <= j < i ==> first[j]@ == second[j]@,
        decreases first.len() - i,
    {
        if !equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn optional_texts_match(first: Option<String>, second: Option<String>) -> bool {
    match (first, second) { (None, None) => true, (Some(a), Some(b)) => a@ == b@, _ => false }
}

pub fn same_optional_text(first: &Option<String>, second: &Option<String>) -> (same: bool)
    ensures same == optional_texts_match(*first, *second),
{
    match (first, second) { (None, None) => true,
        (Some(a), Some(b)) => equal_text(a.as_str(), b.as_str()), _ => false }
}

pub open spec fn matching_artifact(first: &Artifact, second: &Artifact) -> bool {
    first.package_id@ == second.package_id@ && first.manifest_path@ == second.manifest_path@
        && first.target.name@ == second.target.name@ && first.target.src_path@ == second.target.src_path@
        && text_lists_match(first.target.kind@, second.target.kind@)
        && text_lists_match(first.target.crate_types@, second.target.crate_types@)
        && text_lists_match(first.filenames@, second.filenames@)
        && optional_texts_match(first.executable, second.executable) && first.fresh == second.fresh
}

pub fn same_artifact(first: &Artifact, second: &Artifact) -> (same: bool)
    ensures same == matching_artifact(first, second),
{
    equal_text(first.package_id.as_str(), second.package_id.as_str())
        && equal_text(first.manifest_path.as_str(), second.manifest_path.as_str())
        && equal_text(first.target.name.as_str(), second.target.name.as_str())
        && equal_text(first.target.src_path.as_str(), second.target.src_path.as_str())
        && same_text_list(&first.target.kind, &second.target.kind)
        && same_text_list(&first.target.crate_types, &second.target.crate_types)
        && same_text_list(&first.filenames, &second.filenames)
        && same_optional_text(&first.executable, &second.executable) && first.fresh == second.fresh
}

pub fn same_content(first: &Identity, second: &Identity) -> (same: bool)
    ensures same == (first.sha256@ == second.sha256@ && first.length == second.length
        && first.length > 0 && artifact_identity::digest_format(vstd::utf8::encode_utf8(first.sha256@))),
{
    equal_text(first.sha256.as_str(), second.sha256.as_str()) && first.length == second.length
        && first.length > 0 && artifact_identity::valid_sha256(first.sha256.as_str().as_bytes())
}

pub open spec fn matching_identity(first: &Identity, second: &Identity) -> bool {
    first.path@ == second.path@ && first.sha256@ == second.sha256@ && first.length == second.length
        && first.path@.len() > 0 && first.length > 0
        && artifact_identity::digest_format(vstd::utf8::encode_utf8(first.sha256@))
}

pub fn same_identity(first: &Identity, second: &Identity) -> (same: bool)
    ensures same == matching_identity(first, second),
{
    equal_text(first.path.as_str(), second.path.as_str())
        && equal_text(first.sha256.as_str(), second.sha256.as_str()) && first.length == second.length
        && !first.path.as_str().is_empty() && first.length > 0
        && artifact_identity::valid_sha256(first.sha256.as_str().as_bytes())
}

pub open spec fn matching_identities(first: Seq<Identity>, second: Seq<Identity>) -> bool {
    first.len() > 0 && first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> matching_identity(&first[i], &second[i])
}

pub fn same_identities(first: &Vec<Identity>, second: &Vec<Identity>) -> (same: bool)
    ensures same == matching_identities(first@, second@),
{
    if first.len() == 0 || first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant 0 <= i <= first.len() == second.len(), first.len() > 0,
            forall|j: int| 0 <= j < i ==> matching_identity(&first[j], &second[j]),
        decreases first.len() - i,
    {
        if !same_identity(&first[i], &second[i]) { return false; }
        i += 1;
    }
    true
}

pub fn retained_header(schema: &str, inputs: &str, expected: &str, exit: i32, fresh: bool,
    dependency: &[u8], current: &[u8]) -> (accepted: bool)
    ensures accepted == (schema@ == "chrono-library-proof/v1"@ && inputs@ == expected@
        && exit == 0 && !fresh && dependency@ == current@),
{
    equal_text(schema, "chrono-library-proof/v1") && equal_text(inputs, expected)
        && exit == 0 && !fresh && equal_bytes(dependency, current)
}

pub open spec fn crate_name_byte(byte: u8) -> bool {
    (65 <= byte <= 90) || (97 <= byte <= 122) || (48 <= byte <= 57) || byte == 95
}

pub fn library_name(name: &[u8]) -> (valid: bool)
    ensures valid == (name@.len() > 0 && forall|i: int| 0 <= i < name@.len() ==> crate_name_byte(name@[i])),
{
    if name.len() == 0 { return false; }
    let mut i = 0;
    while i < name.len()
        invariant 0 <= i <= name.len(), name.len() > 0,
            forall|j: int| 0 <= j < i ==> crate_name_byte(name@[j]),
        decreases name.len() - i,
    {
        if !((65 <= name[i] && name[i] <= 90) || (97 <= name[i] && name[i] <= 122)
            || (48 <= name[i] && name[i] <= 57) || name[i] == 95) { return false; }
        i += 1;
    }
    true
}

pub open spec fn fingerprint_byte(byte: u8) -> bool { (48 <= byte <= 57) || (97 <= byte <= 102) }

pub fn fingerprint(bytes: &[u8]) -> (valid: bool)
    ensures valid == (bytes@.len() > 0 && forall|i: int| 0 <= i < bytes@.len() ==> fingerprint_byte(bytes@[i])),
{
    if bytes.len() == 0 { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> fingerprint_byte(bytes@[j]),
        decreases bytes.len() - i,
    {
        if !((48 <= bytes[i] && bytes[i] <= 57) || (97 <= bytes[i] && bytes[i] <= 102)) { return false; }
        i += 1;
    }
    true
}

/// The owner command receives the exact mode, output and remaining source arguments.
pub fn owner_source_arguments<'a>(args: &'a [String]) -> (out: Option<(&'a str, &'a str, &'a [String])>)
    ensures out.is_some() == (args.len() >= 3 && args@[0]@ == "check-compiler-sources"@),
        match out {
            Some((kind, output, sources)) => kind@ == args@[1]@ && output@ == args@[2]@
                && sources@ == args@.skip(3),
            None => true,
        },
{
    if args.len() < 3 || !equal_text(args[0].as_str(), "check-compiler-sources") { return None; }
    Some((args[1].as_str(), args[2].as_str(), slice_subrange(args, 3, args.len())))
}

pub fn nonempty_artifact(regular: bool, length: u64) -> (present: bool)
    ensures present == (regular && length > 0),
{ regular && length > 0 }

pub fn one_successful_proof(rows: &[Option<bool>]) -> (accepted: bool)
    ensures accepted == (rows.len() == 1 && rows@[0] == Some(true)),
{
    successful_proofs(rows, 1)
}

pub fn successful_proofs(rows: &[Option<bool>], expected: usize) -> (accepted: bool)
    ensures accepted == (rows.len() == expected
        && forall|i: int| 0 <= i < rows.len() ==> rows@[i] == Some(true)),
{
    if rows.len() != expected { return false; }
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(), rows.len() == expected,
            forall|j: int| 0 <= j < i ==> rows@[j] == Some(true),
        decreases rows.len() - i,
    {
        if rows[i] != Some(true) { return false; }
        i += 1;
    }
    true
}

pub fn archive_source_matches(first: bool, digest: &str, expected_digest: &str,
    length: &str, expected_length: &str) -> (matched: bool)
    ensures matched == (first && digest@ == expected_digest@ && length@ == expected_length@),
{
    first && equal_text(digest, expected_digest) && equal_text(length, expected_length)
}

pub fn all_present(present: &Vec<bool>) -> (complete: bool)
    ensures complete == (present.len() > 0 && forall|i: int| 0 <= i < present.len() ==> present[i]),
{
    if present.len() == 0 { return false; }
    let mut i = 0;
    while i < present.len()
        invariant 0 <= i <= present.len(), present.len() > 0,
            forall|j: int| 0 <= j < i ==> present[j],
        decreases present.len() - i,
    {
        if !present[i] { return false; }
        i += 1;
    }
    true
}

pub open spec fn dependency_name(name: Seq<u8>, prefix: Seq<u8>) -> bool {
    let hash = name.subrange(prefix.len() as int, name.len() - 2);
    prefix.len() > 0 && name.len() >= 2 && prefix.len() < name.len() - 2
        && name.take(prefix.len() as int) == prefix
        && name[name.len() - 2] == 46 && name[name.len() - 1] == 100
        && forall|i: int| 0 <= i < hash.len() ==> fingerprint_byte(hash[i])
}

pub fn retain_attempt(succeeded: bool, cache_rejected: bool) -> (retain: bool)
    ensures retain == (!succeeded || cache_rejected),
{ !succeeded || cache_rejected }

pub fn selected_dependency(name: &[u8], prefix: &[u8]) -> (selected: bool)
    ensures selected == dependency_name(name@, prefix@),
{
    if prefix.len() == 0 || name.len() < 2 || prefix.len() >= name.len() - 2
        || !equal_bytes(slice_subrange(name, 0, prefix.len()), prefix)
        || name[name.len() - 2] != 46 || name[name.len() - 1] != 100 { return false; }
    fingerprint(slice_subrange(name, prefix.len(), name.len() - 2))
}

pub fn output_inventory_within_bound(count: usize) -> (valid: bool)
    ensures valid == (count <= 4096),
{ count <= 4096 }

pub open spec fn library_prefix(directory: Seq<u8>, name: Seq<u8>) -> Seq<u8> {
    directory + seq![47u8, 108u8, 105u8, 98u8] + name + seq![45u8]
}

pub open spec fn library_hash(directory: Seq<u8>, name: Seq<u8>, filenames: Seq<String>) -> Seq<u8> {
    let prefix = library_prefix(directory, name);
    if filenames.len() != 2 { Seq::empty() }
    else {
        let first = vstd::utf8::encode_utf8(filenames[0]@);
        if first.len() < 5 || first.len() - 5 <= prefix.len() { Seq::empty() }
        else { first.subrange(prefix.len() as int, first.len() - 5) }
    }
}

pub open spec fn library_shape(directory: Seq<u8>, name: Seq<u8>, filenames: Seq<String>, executable: bool) -> bool {
    let prefix = library_prefix(directory, name);
    let hash = library_hash(directory, name, filenames);
    !executable && filenames.len() == 2 && name.len() > 0
        && (forall|i: int| 0 <= i < name.len() ==> crate_name_byte(name[i]))
        && hash.len() > 0 && (forall|i: int| 0 <= i < hash.len() ==> fingerprint_byte(hash[i]))
        && vstd::utf8::encode_utf8(filenames[0]@) == prefix + hash + seq![46u8, 114u8, 108u8, 105u8, 98u8]
        && vstd::utf8::encode_utf8(filenames[1]@) == prefix + hash + seq![46u8, 114u8, 109u8, 101u8, 116u8, 97u8]
}

pub struct LibraryPaths { pub dependency: Vec<u8>, pub outputs: Vec<Vec<u8>> }

fn extend(bytes: &mut Vec<u8>, suffix: &[u8])
    ensures bytes@ == old(bytes)@ + suffix@,
{
    bytes.extend_from_slice(suffix);
    assert(bytes@ =~= old(bytes)@ + suffix@);
}

/// 从同一 Cargo JSON 的 rlib/rmeta 对取得 Rust 1.95 的原 dep-info 地址，
/// 拒绝其它目录、别名、混配 fingerprint、未知字节和非库目标。
pub fn compiler_library_paths(directory: &[u8], name: &[u8], filenames: &Vec<String>, executable: bool) -> (out: Option<LibraryPaths>)
    ensures out.is_some() == library_shape(directory@, name@, filenames@, executable),
        out.is_some() ==> out.unwrap().dependency@ == directory@ + seq![47u8] + name@ + seq![45u8]
            + library_hash(directory@, name@, filenames@) + seq![46u8, 100u8],
        out.is_some() ==> out.unwrap().outputs.len() == 2
            && out.unwrap().outputs[0]@ == vstd::utf8::encode_utf8(filenames[0]@)
            && out.unwrap().outputs[1]@ == vstd::utf8::encode_utf8(filenames[1]@),
{
    if executable || filenames.len() != 2 || !library_name(name) { return None; }
    let mut prefix = Vec::new();
    extend(&mut prefix, directory);
    extend(&mut prefix, &[47u8, 108u8, 105u8, 98u8]);
    extend(&mut prefix, name);
    prefix.push(45u8);
    assert(prefix@ == library_prefix(directory@, name@));
    let first = filenames[0].as_str().as_bytes();
    let second = filenames[1].as_str().as_bytes();
    if first.len() < 5 || first.len() - 5 <= prefix.len() { return None; }
    let hash = slice_subrange(first, prefix.len(), first.len() - 5);
    assert(hash@ == library_hash(directory@, name@, filenames@));
    if !fingerprint(hash) { return None; }
    let mut expected_first = prefix.clone();
    extend(&mut expected_first, hash);
    extend(&mut expected_first, &[46u8, 114u8, 108u8, 105u8, 98u8]);
    let mut expected_second = prefix;
    extend(&mut expected_second, hash);
    extend(&mut expected_second, &[46u8, 114u8, 109u8, 101u8, 116u8, 97u8]);
    if !equal_bytes(first, expected_first.as_slice()) || !equal_bytes(second, expected_second.as_slice()) { return None; }
    let mut dependency = Vec::new();
    extend(&mut dependency, directory);
    dependency.push(47u8);
    extend(&mut dependency, name);
    dependency.push(45u8);
    extend(&mut dependency, hash);
    extend(&mut dependency, &[46u8, 100u8]);
    let mut outputs = Vec::new();
    outputs.push(slice_to_vec(first));
    outputs.push(slice_to_vec(second));
    Some(LibraryPaths { dependency, outputs })
}

/// A single-job cargo-verus invocation prints an anonymous proof immediately
/// before its compiler artifact. The caller supplies a stable index per requested target.
pub enum SerialEvent {
    Proof,
    Artifact { selected: Option<usize>, fresh: bool },
    Note,
    Error,
    Finished { success: bool },
}

pub struct SerialProofs {
    seen: Vec<bool>,
    pending: bool,
    finished: bool,
}

pub open spec fn serial_accepts(seen: Seq<bool>, pending: bool, finished: bool, event: SerialEvent) -> bool {
    !finished && match event {
        SerialEvent::Proof => !pending,
        SerialEvent::Artifact { selected: Some(i), fresh } =>
            pending && !fresh && i < seen.len() && !seen[i as int],
        SerialEvent::Artifact { selected: None, .. } | SerialEvent::Note => !pending,
        SerialEvent::Error => false,
        SerialEvent::Finished { success } => !pending && success,
    }
}

impl SerialProofs {
    pub closed spec fn bound(&self) -> Seq<bool> { self.seen@ }
    pub closed spec fn waiting(&self) -> bool { self.pending }
    pub closed spec fn ended(&self) -> bool { self.finished }

    pub fn new(count: usize) -> (state: Self)
        ensures state.bound() == Seq::new(count as nat, |i: int| false),
            !state.waiting(), !state.ended(),
    {
        let seen = vec![false; count];
        assert(seen@ =~= Seq::new(count as nat, |i: int| false));
        Self { seen, pending: false, finished: false }
    }

    /// Any rejection leaves the state unchanged. The reader stops on that record.
    pub fn accept(&mut self, event: SerialEvent) -> (ok: bool)
        ensures
            ok == serial_accepts(old(self).bound(), old(self).waiting(), old(self).ended(), event),
            self.bound() == match event {
                SerialEvent::Artifact { selected: Some(i), .. } if ok => old(self).bound().update(i as int, true),
                _ => old(self).bound(),
            },
            self.waiting() == if !ok { old(self).waiting() } else { event is Proof },
            self.ended() == (old(self).ended() || (ok && event is Finished)),
    {
        if self.finished { return false; }
        match event {
            SerialEvent::Proof if !self.pending => { self.pending = true; }
            SerialEvent::Artifact { selected: Some(i), fresh } => {
                if !self.pending || fresh || i >= self.seen.len() || self.seen[i] { return false; }
                self.seen.set(i, true);
                self.pending = false;
            }
            SerialEvent::Artifact { selected: None, .. } | SerialEvent::Note => {
                if self.pending { return false; }
            }
            SerialEvent::Finished { success: true } if !self.pending => { self.finished = true; }
            _ => return false,
        }
        true
    }

    pub fn complete(&self) -> (ok: bool)
        ensures ok == (self.ended() && !self.waiting() && self.bound().len() > 0
            && forall|i: int| 0 <= i < self.bound().len() ==> self.bound()[i]),
    {
        if !self.finished || self.pending || self.seen.len() == 0 { return false; }
        let mut i = 0;
        while i < self.seen.len()
            invariant 0 <= i <= self.seen.len(), forall|j: int| 0 <= j < i ==> self.seen@[j],
            decreases self.seen.len() - i,
        {
            if !self.seen[i] { return false; }
            i += 1;
        }
        true
    }
}

/// matches 由调用方核对点名库／原证明或最终二进制的实际字节后产生。
pub enum Event {
    Proof { accepted: bool },
    Library { fresh: bool, matches: bool },
    Binary { matches: bool },
    Finished { success: bool },
    OtherArtifact,
    Note,
    Error,
}

pub struct State { pub phase: u8 }

pub struct TargetIdentity { pub manifest: String, pub source: String, pub name: String }
pub struct BuildTargets { pub library: TargetIdentity, pub binary: TargetIdentity, pub published: String }

pub open spec fn target_matches(artifact: &Artifact, target: &TargetIdentity, kind: Seq<char>) -> bool {
    selected_target(artifact, target.manifest@, target.source@, target.name@, kind)
}

pub fn target_selected(artifact: &Artifact, target: &TargetIdentity, kind: &str) -> (selected: bool)
    ensures selected == target_matches(artifact, target, kind@),
{ artifact_selected(artifact, target.manifest.as_str(), target.source.as_str(), target.name.as_str(), kind) }

pub open spec fn record_event(record: &Record, targets: &BuildTargets) -> Event {
    match record {
        Record::Proof(report) => Event::Proof { accepted: *report },
        Record::Cargo(Cargo::Artifact(artifact)) => {
            if target_matches(artifact, &targets.library, "lib"@) {
                Event::Library { fresh: artifact.fresh, matches: true }
            } else if target_matches(artifact, &targets.binary, "bin"@) {
                Event::Binary { matches: published_binary(artifact, targets.published@) }
            } else { Event::OtherArtifact }
        },
        Record::Cargo(Cargo::Message { message }) =>
            if message.level@ != "warning"@ && message.level@ != "note"@ && message.level@ != "help"@
                { Event::Error } else { Event::Note },
        Record::Cargo(Cargo::Script { .. }) => Event::Note,
        Record::Cargo(Cargo::Finished { success }) => Event::Finished { success: *success },
    }
}

pub fn inspect(record: &Record, targets: &BuildTargets) -> (event: Event)
    ensures event == record_event(record, targets),
{
    match record {
        Record::Proof(report) => Event::Proof { accepted: *report },
        Record::Cargo(Cargo::Artifact(artifact)) => {
            if target_selected(artifact, &targets.library, "lib") {
                Event::Library { fresh: artifact.fresh, matches: true }
            } else if target_selected(artifact, &targets.binary, "bin") {
                Event::Binary { matches: binary_shape(artifact, targets.published.as_str()) }
            } else { Event::OtherArtifact }
        },
        Record::Cargo(Cargo::Message { message }) =>
            if diagnostic_rejected(message.level.as_str()) { Event::Error } else { Event::Note },
        Record::Cargo(Cargo::Script { .. }) => Event::Note,
        Record::Cargo(Cargo::Finished { success }) => Event::Finished { success: *success },
    }
}

pub open spec fn trace(records: Seq<Record>, targets: &BuildTargets) -> Option<u8>
    decreases records.len(),
{
    if records.len() == 0 { Some(0u8) }
    else { match trace(records.drop_last(), targets) {
        Some(phase) => next_phase(phase, record_event(&records.last(), targets)),
        None => None,
    } }
}

/// 完整原输出在任何缓存恢复或文件发布前核对；错误不能被清理／重跑覆盖。
pub fn complete_records(records: &Vec<Record>, targets: &BuildTargets) -> (accepted: bool)
    ensures accepted == (trace(records@, targets) == Some(4u8)),
{
    let mut state = State::new();
    let mut i = 0;
    while i < records.len()
        invariant 0 <= i <= records.len(),
            trace(records@.take(i as int), targets) == Some(state.phase),
        decreases records.len() - i,
    {
        proof {
            assert(records@.take(i as int + 1).drop_last() =~= records@.take(i as int));
            assert(records@.take(i as int + 1).last() == records@[i as int]);
        }
        let event = inspect(&records[i], targets);
        state = match state.step(event) {
            Ok(next) => next,
            Err(_) => {
                proof { failed_trace_stays_failed(records@, targets, i as int + 1); }
                return false;
            },
        };
        i += 1;
    }
    proof { assert(records@.take(i as int) =~= records@); }
    state.accepted()
}

proof fn failed_trace_stays_failed(records: Seq<Record>, targets: &BuildTargets, prefix: int)
    requires 0 <= prefix <= records.len(), trace(records.take(prefix), targets).is_none(),
    ensures trace(records, targets).is_none(),
    decreases records.len() - prefix,
{
    if prefix < records.len() {
        assert(records.drop_last().take(prefix) =~= records.take(prefix));
        failed_trace_stays_failed(records.drop_last(), targets, prefix);
    } else { assert(records.take(prefix) =~= records); }
}

pub open spec fn next_phase(phase: u8, event: Event) -> Option<u8> {
    if phase >= 4 { None }
    else {
        match event {
            Event::Proof { accepted } => if phase == 0 && accepted { Some(1u8) } else { None },
            Event::Library { fresh, matches } =>
                if matches && ((fresh && phase == 0) || (!fresh && phase == 1)) { Some(2u8) }
                else { None },
            Event::Binary { matches } => if phase == 2 && matches { Some(3u8) } else { None },
            Event::Finished { success } => if phase == 3 && success { Some(4u8) } else { None },
            Event::OtherArtifact => Some(phase),
            Event::Note => Some(phase),
            Event::Error => None,
        }
    }
}

impl State {
    pub fn new() -> (state: Self)
        ensures state.phase == 0,
    { Self { phase: 0 } }

    pub fn accepted(&self) -> (accepted: bool)
        ensures accepted == (self.phase == 4),
    { self.phase == 4 }

    /// 每条原记录只推进一次；未知、失败及重复终态不能被后续成功覆盖。
    pub fn step(&self, event: Event) -> (out: Result<Self, u8>)
        ensures out.is_ok() == next_phase(self.phase, event).is_some(),
            out.is_ok() ==> out.unwrap().phase == next_phase(self.phase, event).unwrap(),
            match out { Err(code) => code == self.phase, _ => true },
    {
        if self.phase >= 4 { return Err(self.phase); }
        let phase = match event {
            Event::Proof { accepted } => {
                if self.phase != 0 || !accepted { return Err(self.phase); }
                1u8
            },
            Event::Library { fresh, matches } => {
                if !matches || !((fresh && self.phase == 0) || (!fresh && self.phase == 1)) {
                    return Err(self.phase);
                }
                2u8
            },
            Event::Binary { matches } => {
                if self.phase != 2 || !matches { return Err(self.phase); }
                3u8
            },
            Event::Finished { success } => {
                if self.phase != 3 || !success { return Err(self.phase); }
                4u8
            },
            Event::OtherArtifact => {
                self.phase
            },
            Event::Note => self.phase,
            Event::Error => { return Err(self.phase); },
        };
        Ok(Self { phase })
    }
}

}
