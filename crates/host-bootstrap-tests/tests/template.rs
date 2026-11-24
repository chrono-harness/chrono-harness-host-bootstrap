use chrono_host_bootstrap::template::prepare;
use serde_json::{Value, json};
use std::{fs, path::Path};

const ANSWERS: &str = "# comments survive\r\nrepository: ''\r\nlocal: 'keep: me'\ndev_branch: ''";

fn host() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let settings = temp.path().join(".chrono-harness");
    fs::create_dir(&settings).unwrap();
    fs::write(settings.join("template-answers.yml"), ANSWERS).unwrap();
    fs::write(
        settings.join("pr-policy.json"),
        "{\"required_checks\":[\"CI\"]}",
    )
    .unwrap();
    fs::write(settings.join("pr.json"), "previous PR settings").unwrap();
    temp
}

fn run(
    root: &Path,
    event: Option<&Path>,
    commands: &[(&[&str], Result<&str, &str>)],
) -> Result<String, String> {
    let mut calls = commands.iter();
    let result = prepare(root, event, |program, args| {
        let (expected, response) = calls.next().expect("unexpected invocation");
        assert_eq!(program, expected[0]);
        assert_eq!(args, &expected[1..]);
        response.map(str::to_owned).map_err(str::to_owned)
    });
    assert!(calls.next().is_none());
    result
}
const ORIGIN: &[&str] = &["git", "config", "--get", "remote.origin.url"];
const DEFAULT: &[&str] = &["git", "ls-remote", "--symref", "origin", "HEAD"];
const SELECTION: &[&str] = &["mise", "ls", "--current", "--json"];
const SYNC: &[&str] = &[
    "mise",
    "exec",
    "--",
    "chrono-instructions",
    "sync-rules",
    "--host-root",
    ".",
];

#[test]
fn event_for_origin_selects_pr_and_rules_and_preserves_other_answers() {
    let host = host();
    let root = host.path();
    let event = root.join("event.json");
    fs::write(
        &event,
        r#"{"repository":{"full_name":"owner/a","default_branch":"project/dev"}}"#,
    )
    .unwrap();
    run(
        root,
        Some(&event),
        &[
            (ORIGIN, Ok("https://github.com/Owner/A.git")),
            (
                SELECTION,
                Ok(r#"{"http:chrono-pr":[],"http:chrono-instructions":[]}"#),
            ),
            (SYNC, Ok("synced")),
        ],
    )
    .unwrap();
    let pr: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/pr.json")).unwrap()).unwrap();
    assert_eq!(
        pr,
        json!({"base":"project/dev","repository":"Owner/A","required_checks":["CI"]})
    );
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/template-answers.yml")).unwrap(),
        "# comments survive\r\nrepository: \"Owner/A\"\r\nlocal: 'keep: me'\ndev_branch: \"project/dev\""
    );
}

#[test]
fn foreign_event_uses_remote_head_and_disabling_products_clears_old_pr_settings() {
    let host = host();
    let root = host.path();
    let event = root.join("event.json");
    fs::write(
        &event,
        r#"{"repository":{"full_name":"elsewhere/wrong","default_branch":"wrong"}}"#,
    )
    .unwrap();
    for origin in [
        "git@github.com:Other/B.git",
        "ssh://git@ssh.github.com:443/Other/B.git/",
        "https://GitHub.COM/Other/B",
    ] {
        let result = run(
            root,
            Some(&event),
            &[
                (ORIGIN, Ok(origin)),
                (DEFAULT, Ok("ref: refs/heads/trunk\tHEAD\nabcdef\tHEAD\n")),
                (SELECTION, Ok("{}")),
            ],
        )
        .unwrap();
        assert_eq!(result, "Configured Other/B on trunk");
        assert!(!root.join(".chrono-harness/pr.json").exists());
    }
}

#[test]
fn invalid_origin_default_and_sync_failure_preserve_preexisting_files() {
    let host = host();
    let root = host.path();
    for invalid in [
        "/tmp/local",
        "https://github.com.evil/owner/repo",
        "git@github.com:only-owner",
        "https://github.com/owner/repo/tree",
        "https://github.com/./repo",
        "https://github.com/owner/..",
    ] {
        assert!(run(root, None, &[(ORIGIN, Ok(invalid))]).is_err());
    }
    assert!(
        run(
            root,
            None,
            &[
                (ORIGIN, Ok("https://github.com/Owner/A")),
                (DEFAULT, Ok("abc\tHEAD"))
            ]
        )
        .is_err()
    );
    let git = [
        (ORIGIN, Ok("https://github.com/Owner/A")),
        (DEFAULT, Ok("ref: refs/heads/main\tHEAD")),
    ];
    let commands = [
        git.as_slice(),
        &[
            (SELECTION, Ok(r#"{"http:chrono-instructions":[]}"#)),
            (SYNC, Err("sync error")),
        ],
    ]
    .concat();
    assert_eq!(run(root, None, &commands).unwrap_err(), "sync error");
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/template-answers.yml")).unwrap(),
        ANSWERS
    );
    let answers = root.join(".chrono-harness/template-answers.yml");
    fs::write(&answers, "repository: x\nrepository: y\ndev_branch: dev").unwrap();
    assert!(
        run(
            root,
            None,
            &[git.as_slice(), &[(SELECTION, Ok("{}"))]].concat()
        )
        .is_err()
    );
    assert_eq!(
        fs::read_to_string(&answers).unwrap(),
        "repository: x\nrepository: y\ndev_branch: dev"
    );
    assert_eq!(
        fs::read_to_string(root.join(".chrono-harness/pr.json")).unwrap(),
        "previous PR settings"
    );
}
