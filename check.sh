#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/host-bootstrap/Cargo.toml
product=crates/host-bootstrap/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/cargo.jsonl"
cargo fmt --check --manifest-path crates/host-bootstrap-tests/Cargo.toml
cargo test --locked --manifest-path crates/host-bootstrap-tests/Cargo.toml
# Select the candidate from this Cargo invocation, including nondefault target directories.
artifact() {
  jq -ers --arg manifest "$(pwd -P)/$product" --arg kind "$1" '
    [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest and
                  .target.kind == [$kind]) |
     if $kind == "bin" then .executable else .filenames[] | select(endswith(".rmeta")) end]
    | if length == 1 then .[0] else error("expected one bootstrap artifact") end
  ' "$scratch/cargo.jsonl"
}
bootstrap=$(artifact bin)
metadata=$(artifact lib)
filename=${metadata##*/}
stem=${filename#lib}
library=${metadata%/*}/${stem%.rmeta}

src=crates/host-bootstrap/src
tests=crates/host-bootstrap-tests/tests
binding_modules=(artifact_identity.rs construction.rs lifecycle.rs owner.rs profile.rs execution.rs original.rs)
native_sources=(src/lib.rs src/verified_bindings.rs src/verified_cargo.rs src/cargo_proofs.rs src/proved_build.rs)
"$bootstrap" check-compiler-sources native "$library" "${native_sources[@]}" "${binding_modules[@]/#/src/}"

prove() {
  "$bootstrap" run-compiler-proof "$scratch" "$1" "${VERUS:-verus}" "$2"
}
collect() {
  local operation=$1
  shift
  "$bootstrap" collect-compiler-proof "$scratch" "$operation" "${VERUS:-verus}" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" || {
      echo "E_VERUS_COVERAGE: $operation" >&2
      return 1
    }
}

prove verify.bootstrap-bindings "$src/verified_bindings.rs"
prove verify.bootstrap-bindings-tests "$tests/verified_bindings.rs"
prove verify.bootstrap-cargo-tests "$tests/verified_cargo.rs"

included="$tests/../../host-bootstrap/src"
collect verify.bootstrap-bindings "$src/verified_bindings.rs" "${binding_modules[@]/#/$src/}"
collect verify.bootstrap-bindings-tests "$tests/verified_bindings.rs" "$included/verified_bindings.rs" \
  "${binding_modules[@]/#/$included/}" "$tests/support/expect.rs" \
  "$tests/proof_support/host-bootstrap/construction.rs" "$tests/proof_support/host-bootstrap/owner.rs" \
  "$tests/proof_support/host-bootstrap/original.rs"
collect verify.bootstrap-cargo-tests "$tests/verified_cargo.rs" "$included/verified_cargo.rs" \
  "$included/artifact_identity.rs" "$tests/support/expect.rs"
