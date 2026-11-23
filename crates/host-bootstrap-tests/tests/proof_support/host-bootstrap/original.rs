#[cfg(verus_keep_ghost)]
use super::policy::execution::operation_argument;

use super::expect;
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::execution::operation_argument;
use vstd::prelude::*;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn only_the_declared_argument_uses_the_current_attempt_output() {
    let output = ".chrono-harness/state/bootstrap-current/0.output";
    let bound = operation_argument("{operation-output}", output);
    let literal = operation_argument("verus-download.headers", output);
    proof {
        reveal_strlit("{operation-output}");
        reveal_strlit("verus-download.headers");
        assert("{operation-output}"@.len() != "verus-download.headers"@.len());
    }
    expect(super::policy::artifact_identity::equal_text(bound, output));
    expect(super::policy::artifact_identity::equal_text(literal, "verus-download.headers"));
}
}
