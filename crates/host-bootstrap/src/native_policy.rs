//! Decisions used by native scratch disposal. Git, clock and lock observations come from the caller.
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Translate a Unix child observation into the exit code returned to the calling shell.
pub fn shell_exit(code: Option<i32>, signal: Option<i32>) -> (exit: u8)
    ensures exit as int == match code {
        Some(value) => if 0 <= value <= 255 { value as int } else { 1int },
        None => match signal {
            Some(value) => if 1 <= value <= 127 { 128 + value as int } else { 1int },
            None => 1int,
        },
    },
{
    match super::process_exit(code, signal) {
        Some(status) if 0 <= status && status <= 255 => status as u8,
        Some(status) if code.is_none() && -127 <= status && status < 0 => (128 - status) as u8,
        _ => 1,
    }
}

/// Skip unrelated names and protected Git inputs before opening a candidate.
pub fn candidate(generated: bool, git_protected: bool) -> (allow: bool)
    ensures allow == (generated && !git_protected),
{ generated && !git_protected }

/// None is used only by the successful command that owns the directory.
pub fn time_ready(keep: Option<u64>, elapsed: Option<u64>) -> (ready: bool)
    ensures ready == match keep {
        None => true,
        Some(seconds) => seconds > 0 && match elapsed { Some(age) => age >= seconds, None => false },
    },
{
    match (keep, elapsed) {
        (None, _) => true,
        (Some(seconds), Some(age)) => seconds > 0 && age >= seconds,
        _ => false,
    }
}

/// A partial signature is sufficient only while the marker is the sole entry.
/// A missing signature can be recovered only for an empty directory.
pub fn marker_owned(complete: bool, matching_prefix_only: bool, missing_empty: bool) -> (owned: bool)
    ensures owned == (complete || matching_prefix_only || missing_empty),
{ complete || matching_prefix_only || missing_empty }

/// All three observations are made under the business gate before deleting members.
pub fn removable(exclusively_held: bool, owned: bool, same_directory: bool) -> (allowed: bool)
    ensures allowed == (exclusively_held && owned && same_directory),
{ exclusively_held && owned && same_directory }
}
