#[cfg(verus_keep_ghost)]
use super::policy::{lifecycle as stage_lifecycle, owner as stage_owner, profile as stage_profile};

use super::expect;
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::{
    lifecycle as stage_lifecycle, owner as stage_owner, profile as stage_profile,
};
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn stage_recovery_requires_the_same_source_and_installed_bytes() {
    proof {
        reveal_strlit("stage"); reveal_strlit("earlier");
        reveal_strlit("first"); reveal_strlit("later");
        assert("stage"@.len() != "earlier"@.len());
        assert("first"@[0] != "later"@[0]);
    }
    let exact = stage_owner::stage_reuse_matches("stage", "first", "stage", "first");
    let other_slot = stage_owner::stage_reuse_matches("earlier", "first", "stage", "first");
    let other_digest = stage_owner::stage_reuse_matches("stage", "later", "stage", "first");
    let state = stage_profile::ordinary_state_input(false, true);
    let config = stage_profile::ordinary_state_input(false, false);
    let named = stage_profile::ordinary_state_input(true, true);
    let copied = stage_lifecycle::qualified_copy_matches(true, true, 16, 16);
    let stale = stage_lifecycle::qualified_copy_matches(false, true, 16, 16);
    let replaced = stage_lifecycle::qualified_copy_matches(true, false, 16, 16);
    let truncated = stage_lifecycle::qualified_copy_matches(true, true, 15, 16);
    expect(exact && !other_slot && !other_digest && state && !config && !named);
    expect(copied && !stale && !replaced && !truncated);
}
}
