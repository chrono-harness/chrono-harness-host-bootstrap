#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# == 1 && $1 == --native ]]; then
  exec chrono-host-bootstrap native-scratch run /bin/bash ./check.sh --native-in-directory
fi
if [[ $# -ne 1 || $1 != --native-in-directory ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
scratch=${CHRONO_NATIVE_SCRATCH:?missing native check directory}
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
"$bootstrap" ci-install-table-v1 manager > "$scratch/manager.expected"
sed -n '/^ci_install_choice() {$/,/^}$/p' .chrono-harness/ci/install.sh > "$scratch/manager.actual"
cmp "$scratch/manager.expected" "$scratch/manager.actual"
python3 -B .chrono-harness/ci/test_install.py
metadata=$(artifact lib)
filename=${metadata##*/}
stem=${filename#lib}
library=${metadata%/*}/${stem%.rmeta}

src=crates/host-bootstrap/src
tests=crates/host-bootstrap-tests/tests
binding_modules=(native_policy.rs verified_cargo.rs artifact_identity.rs construction.rs lifecycle.rs owner.rs profile.rs environment.rs execution.rs files.rs original.rs registered_methods.rs report_query.rs source.rs template.rs)
native_sources=(src/native_scratch.rs src/lib.rs src/verified_bindings.rs src/cargo_proofs.rs src/proved_build.rs)
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
  "$tests/proof_support/host-bootstrap/original.rs" "$tests/bindings.rs"
collect verify.bootstrap-cargo-tests "$tests/verified_cargo.rs" "$included/verified_cargo.rs" \
  "$included/artifact_identity.rs" "$tests/support/expect.rs" "$tests/registered_methods.rs"
