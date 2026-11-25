//! Decisions used by native scratch disposal. Git, clock and lock observations come from the caller.
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// Skip unrelated names and protected Git inputs before opening a candidate.
pub fn candidate(generated: bool, tracked: bool) -> (allow: bool)
    ensures allow == (generated && !tracked),
{ generated && !tracked }

/// None is used only by the successful command that owns the directory.
pub fn time_ready(keep: Option<u64>, elapsed: Option<u64>) -> (ready: bool)
    ensures ready == match keep {
        None => true,
        Some(seconds) => seconds > 0 && elapsed matches Some(age) && age >= seconds,
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
