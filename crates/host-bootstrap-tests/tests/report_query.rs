use chrono_host_bootstrap::{bindings::Environment, report_query};
use serde_json::{Value, json};
use std::path::Path;

fn query(input: Value) -> Result<Value, String> {
    let bytes = serde_json::to_vec(&input).unwrap();
    report_query::read(Path::new("."), &Environment::new(), bytes.as_slice())
}

#[test]
fn retained_report_must_identify_the_requested_git_revision() {
    let commit = "1".repeat(40);
    let tree = "2".repeat(40);
    for (other_commit, other_tree) in [
        ("3".repeat(40), tree.clone()),
        (commit.clone(), "4".repeat(40)),
    ] {
        let result = query(json!({
            "operation":"retained", "profile":".chrono-harness/ci/build.json",
            "platform":{"system":"Linux","machine":"x86_64"},
            "revision":{"commit":commit,"tree":tree},
            "source":{"commit":other_commit,"tree":other_tree,"dirty":false,"error":null},
            "report":"{}", "directory":"/retained"
        }));
        assert!(
            result
                .unwrap_err()
                .contains("differs from requested revision")
        );
    }
}

#[test]
fn stream_listing_requires_a_complete_original_report() {
    assert!(query(json!({"operation":"streams","report":"{\"operations\":[]}"})).is_err());
    assert!(query(json!({"operation":"platform","profile":"extra"})).is_err());
    let actual = query(json!({"operation":"platform"})).unwrap();
    assert!(actual["machine"].is_string() && actual["system"].is_string());
}
