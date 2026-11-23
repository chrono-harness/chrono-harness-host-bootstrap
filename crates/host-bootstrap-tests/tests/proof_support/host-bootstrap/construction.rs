#[cfg(verus_keep_ghost)]
use super::policy::construction::published_slot;

use super::expect;
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::construction::published_slot;
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
}
