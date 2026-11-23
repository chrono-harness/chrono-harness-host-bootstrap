//! Compare the producer's artifact bindings and admit their recorded digests and Git IDs.
use vstd::contrib::exec_spec::ExecSpecEq;
use vstd::prelude::*;

verus! {

pub fn equal_text(a: &str, b: &str) -> (equal: bool)
    ensures equal == (a@ == b@),
{ <&str>::exec_eq(a, b) }

pub fn equal_bytes(a: &[u8], b: &[u8]) -> (equal: bool)
    ensures equal == (a@ == b@),
{
    proof {
        assert(a.deep_view() =~= a@);
        assert(b.deep_view() =~= b@);
    }
    <&[u8]>::exec_eq(a, b)
}

pub open spec fn hash_byte(b: u8) -> bool {
    (48 <= b <= 57) || (97 <= b <= 102)
}

#[verifier::inline]
pub open spec fn digest_format(bytes: Seq<u8>) -> bool {
    bytes.len() == 64 && forall|i: int| 0 <= i < bytes.len() ==> hash_byte(#[trigger] bytes[i])
}

#[verifier::inline]
pub open spec fn oid(bytes: Seq<u8>) -> bool {
    (bytes.len() == 40 || bytes.len() == 64)
        && (forall|i: int| 0 <= i < bytes.len() ==> hash_byte(#[trigger] bytes[i]))
        && (exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48)
}

fn hexadecimal(bytes: &[u8], require_nonzero: bool) -> (valid: bool)
    ensures valid == ((forall|i: int| 0 <= i < bytes.len() ==> hash_byte(#[trigger] bytes[i]))
        && (!require_nonzero || exists|i: int| 0 <= i < bytes.len() && bytes[i] != 48)),
{
    let mut i = 0;
    let mut nonzero = false;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> hash_byte(#[trigger] bytes[j]),
            nonzero == (exists|j: int| 0 <= j < i && bytes[j] != 48),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !((48 <= b && b <= 57) || (97 <= b && b <= 102)) { return false; }
        nonzero = nonzero || b != 48;
        i += 1;
    }
    !require_nonzero || nonzero
}

pub fn valid_sha256(bytes: &[u8]) -> (valid: bool)
    ensures valid == digest_format(bytes@),
{ bytes.len() == 64 && hexadecimal(bytes, false) }

pub fn valid_oid(bytes: &[u8]) -> (valid: bool)
    ensures valid == oid(bytes@),
{ (bytes.len() == 40 || bytes.len() == 64) && hexadecimal(bytes, true) }

}
