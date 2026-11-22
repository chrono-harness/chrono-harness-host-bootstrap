#[cfg(verus_keep_ghost)]
use super::policy::execution::operation_argument;

#[cfg(verus_keep_ghost)]
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
    assert(bound@ == output@);
    assert(literal@ == "verus-download.headers"@);
}
}
