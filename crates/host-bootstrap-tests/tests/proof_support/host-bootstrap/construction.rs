#[cfg(verus_keep_ghost)]
use super::policy::construction::{complete_outcome, published_slot};

use super::expect;
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::construction::{complete_outcome, published_slot};
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn only_a_successful_run_with_a_declared_slot_publishes_a_current_report() {
    let current = published_slot(Some("core"), true);
    let failed = published_slot(Some("core"), false);
    let run_only = published_slot(None, true);
    let failed_run_only = published_slot(None, false);
    expect(current.is_some() && super::policy::artifact_identity::equal_text(current.unwrap(), "core"));
    expect(failed.is_none() && run_only.is_none() && failed_run_only.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn build_outcome_preserves_the_first_failure_and_rejects_incomplete_success() {
    let original_complete = complete_outcome(Err(37u32), true);
    let original_incomplete = complete_outcome(Err(42u32), false);
    let incomplete = complete_outcome::<u32>(Ok(()), false);
    let complete = complete_outcome::<u32>(Ok(()), true);
    expect(matches!(original_complete, Err(Some(37))));
    expect(matches!(original_incomplete, Err(Some(42))));
    expect(matches!(incomplete, Err(None)));
    expect(matches!(complete, Ok(())));
}
}
