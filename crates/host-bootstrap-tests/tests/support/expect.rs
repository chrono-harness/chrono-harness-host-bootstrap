use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
pub fn expect(value: bool)
    requires value,
{ if !value { panic!(); } }
}
