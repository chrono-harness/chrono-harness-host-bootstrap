//! Bind a green-template checkout to its own GitHub identity and selected products.
#[cfg(verus_keep_ghost)]
use super::byte_prefix;
#[cfg(not(verus_keep_ghost))]
use crate::verified_bindings::byte_prefix;
use vstd::prelude::*;

verus! {
pub struct AnswerFields {
    pub repository: bool,
    pub branch: bool,
}

pub open spec fn starts(bytes: Seq<u8>, key: Seq<u8>) -> bool {
    key.len() <= bytes.len() && forall|i: int| 0 <= i < key.len() ==> bytes[i] == key[i]
}

pub open spec fn ending(bytes: Seq<u8>) -> Seq<char> {
    if bytes.len() == 0 || bytes.last() != 10 { ""@ }
    else if bytes.len() >= 2 && bytes[bytes.len() - 2] == 13 { "\r\n"@ }
    else { "\n"@ }
}

fn line_ending(bytes: &[u8]) -> (suffix: &'static str)
    ensures suffix@ == ending(bytes@),
{
    let n = bytes.len();
    if n == 0 || bytes[n - 1] != 10 { "" }
    else if n >= 2 && bytes[n - 2] == 13 { "\r\n" }
    else { "\n" }
}

impl AnswerFields {
    pub fn complete(&self) -> (ok: bool)
        ensures ok == (self.repository && self.branch),
    { self.repository && self.branch }

    /// Return the two fragments emitted for this physical line, or reject a repeated key.
    pub fn line<'a>(&mut self, line: &'a str, repository: &'a str, branch: &'a str)
        -> (fragments: Option<(&'a str, &'a str)>)
        ensures ({
            let bytes = vstd::utf8::encode_utf8(line@);
            let repo = starts(bytes, vstd::utf8::encode_utf8("repository:"@));
            let dev = !repo && starts(bytes, vstd::utf8::encode_utf8("dev_branch:"@));
            let duplicate = (repo && old(self).repository) || (dev && old(self).branch);
            &&& fragments.is_none() == duplicate
            &&& self.repository == (old(self).repository || repo)
            &&& self.branch == (old(self).branch || dev)
            &&& match fragments {
                Some((text, suffix)) =>
                    text@ == (if repo { repository@ } else if dev { branch@ } else { line@ })
                    && suffix@ == (if repo || dev { ending(bytes) } else { ""@ }),
                None => true,
            }
        }),
    {
        let bytes = line.as_bytes();
        if byte_prefix(bytes, "repository:".as_bytes()) {
            if self.repository { return None; }
            self.repository = true;
            Some((repository, line_ending(bytes)))
        } else if byte_prefix(bytes, "dev_branch:".as_bytes()) {
            if self.branch { return None; }
            self.branch = true;
            Some((branch, line_ending(bytes)))
        } else { Some((line, "")) }
    }
}
}

#[cfg(not(verus_keep_ghost))]
use serde_json::{Map, Value};
#[cfg(not(verus_keep_ghost))]
use std::{
    fs, io,
    path::Path,
    process::{Command, Stdio},
};

#[cfg(not(verus_keep_ghost))]
fn repository(url: &str) -> Result<&str, String> {
    let invalid = || "origin must identify a GitHub owner/repository".to_owned();
    let (authority, path) = if let Some((_, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_once('/').ok_or_else(invalid)?;
        let host = authority.rsplit('@').next().unwrap_or(authority);
        (host.split(':').next().unwrap_or(host), path)
    } else {
        let (authority, path) = url.split_once(':').ok_or_else(invalid)?;
        (authority.rsplit('@').next().unwrap_or(authority), path)
    };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/').ok_or_else(invalid)?;
    if !crate::verified_bindings::github_repository(
        authority.as_bytes(),
        owner.as_bytes(),
        name.as_bytes(),
    ) {
        return Err(invalid());
    }
    Ok(path)
}

#[cfg(not(verus_keep_ghost))]
fn event_branch(path: &Path, repo: &str) -> Result<Option<String>, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let event: Value = serde_json::from_slice(&bytes).map_err(|e| format!("GitHub event: {e}"))?;
    let identity = &event["repository"];
    Ok(identity["full_name"]
        .as_str()
        .filter(|s| s.eq_ignore_ascii_case(repo))
        .and_then(|_| identity["default_branch"].as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_owned))
}

#[cfg(not(verus_keep_ghost))]
fn update_answers(text: &str, repo: &str, branch: &str) -> Result<String, String> {
    let repository = format!(
        "repository: {}",
        serde_json::to_string(repo).map_err(|e| e.to_string())?
    );
    let branch = format!(
        "dev_branch: {}",
        serde_json::to_string(branch).map_err(|e| e.to_string())?
    );
    let invalid = "expected exactly one repository and dev_branch in template-answers.yml";
    let mut fields = AnswerFields {
        repository: false,
        branch: false,
    };
    let mut result = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let (text, suffix) = fields.line(line, &repository, &branch).ok_or(invalid)?;
        result.push_str(text);
        result.push_str(suffix);
    }
    if !fields.complete() {
        return Err(invalid.into());
    }
    Ok(result)
}

#[cfg(not(verus_keep_ghost))]
fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

#[cfg(not(verus_keep_ghost))]
fn write_changed(path: &Path, previous: Option<&[u8]>, next: Option<&[u8]>) -> Result<(), String> {
    if previous == next {
        return Ok(());
    }
    match next {
        Some(bytes) => crate::shared::replace(path, bytes, 0o600),
        None => fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display())),
    }
}

#[cfg(not(verus_keep_ghost))]
pub fn prepare(
    root: &Path,
    event: Option<&Path>,
    mut run: impl FnMut(&str, &[&str]) -> Result<String, String>,
) -> Result<String, String> {
    let origin = run("git", &["config", "--get", "remote.origin.url"])?;
    let repo = repository(origin.trim())?;
    let branch = match event
        .map(|path| event_branch(path, repo))
        .transpose()?
        .flatten()
    {
        Some(branch) => branch,
        None => run("git", &["ls-remote", "--symref", "origin", "HEAD"])?
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .find_map(|(target, name)| {
                (name == "HEAD")
                    .then(|| target.strip_prefix("ref: refs/heads/"))
                    .flatten()
            })
            .filter(|s| !s.is_empty())
            .ok_or("origin does not advertise its default branch")?
            .to_owned(),
    };
    let tools: Map<String, Value> =
        serde_json::from_str(&run("mise", &["ls", "--current", "--json"])?)
            .map_err(|e| format!("mise selection: {e}"))?;
    let policy = root.join(".chrono-harness");
    let answers = policy.join("template-answers.yml");
    let old_answers =
        fs::read_to_string(&answers).map_err(|e| format!("{}: {e}", answers.display()))?;
    let new_answers = update_answers(&old_answers, repo, &branch)?;
    let pr = policy.join("pr.json");
    let old_pr = read_optional(&pr)?;
    let new_pr = if tools.contains_key("http:chrono-pr") {
        let bytes =
            fs::read(policy.join("pr-policy.json")).map_err(|e| format!("pr-policy.json: {e}"))?;
        let mut settings: Map<String, Value> =
            serde_json::from_slice(&bytes).map_err(|e| format!("pr-policy.json: {e}"))?;
        settings.insert("repository".into(), repo.into());
        settings.insert("base".into(), branch.as_str().into());
        let mut output = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
        output.push(b'\n');
        Some(output)
    } else {
        None
    };
    if tools.contains_key("http:chrono-instructions") {
        run(
            "mise",
            &[
                "exec",
                "--",
                "chrono-instructions",
                "sync-rules",
                "--host-root",
                ".",
            ],
        )?;
    }
    write_changed(&pr, old_pr.as_deref(), new_pr.as_deref())?;
    if let Err(error) = write_changed(
        &answers,
        Some(old_answers.as_bytes()),
        Some(new_answers.as_bytes()),
    ) {
        write_changed(&pr, new_pr.as_deref(), old_pr.as_deref())
            .map_err(|rollback| format!("{error}; restoring pr.json: {rollback}"))?;
        return Err(error);
    }
    Ok(format!("Configured {repo} on {branch}"))
}

#[cfg(not(verus_keep_ghost))]
pub fn run() -> Result<String, String> {
    let event = (std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true"))
        .then(|| std::env::var_os("GITHUB_EVENT_PATH"))
        .flatten();
    prepare(
        Path::new("."),
        event.as_deref().map(Path::new),
        |program, args| {
            let output = Command::new(program)
                .args(args)
                .stderr(Stdio::inherit())
                .output()
                .map_err(|e| format!("{program}: {e}"))?;
            if !output.status.success() {
                return Err(format!(
                    "{program} {} exited {}",
                    args.join(" "),
                    output.status
                ));
            }
            String::from_utf8(output.stdout).map_err(|e| format!("{program}: {e}"))
        },
    )
}

/// Produce the finite POSIX dispatch table for a fresh checkout. The selector only
/// matches an observed key. Unknown keys fail before a download or link update.
#[cfg(not(verus_keep_ghost))]
pub fn ci_install_table(check_storage_and_links: bool) -> String {
    use crate::verified_bindings::{
        ci_link, ci_manager, ci_mise_asset, ci_mise_digest, ci_storage,
    };
    use std::fmt::Write;
    let mut table = String::from("ci_install_choice() {\n    case $1 in\n");
    let mut row = |key: String, action: &str| {
        writeln!(table, "        {key}) choice={action} ;;").expect("writing a String");
    };
    for available in [false, true] {
        for hosted in [false, true] {
            row(
                format!("manager-{}{}", u8::from(available), u8::from(hosted)),
                ci_manager(available, hosted),
            );
        }
    }
    for platform in ["Darwin/arm64", "Linux/x86_64"] {
        let asset = ci_mise_asset(platform).expect("registered platform");
        let digest = ci_mise_digest(asset).expect("registered asset");
        row(
            format!("platform-{platform}"),
            &format!("{asset}; digest={digest}"),
        );
    }
    if check_storage_and_links {
        for inside in [false, true] {
            row(format!("storage-{}", u8::from(inside)), ci_storage(inside));
        }
        for symlink in [false, true] {
            for same_target in [false, true] {
                row(
                    format!("link-{}{}", u8::from(symlink), u8::from(same_target)),
                    ci_link(symlink, same_target),
                );
            }
        }
    }
    table.push_str("        *) choice=unsupported ;;\n    esac\n}\n");
    table
}
