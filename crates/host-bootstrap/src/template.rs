//! Bind a green-template checkout to its own GitHub identity and selected products.
use serde_json::{Map, Value};
use std::{
    fs, io,
    path::Path,
    process::{Command, Stdio},
};

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
    let valid = |s: &str| {
        !s.is_empty()
            && s != "."
            && s != ".."
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
    };
    if !(authority.eq_ignore_ascii_case("github.com")
        || authority.eq_ignore_ascii_case("ssh.github.com"))
        || !valid(owner)
        || !valid(name)
    {
        return Err(invalid());
    }
    Ok(path)
}

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

fn update_answers(text: &str, repo: &str, branch: &str) -> Result<String, String> {
    // Only these two scalar lines belong to setup. Preserve comments, ordering and local answers.
    let bindings = [("repository:", repo), ("dev_branch:", branch)];
    let mut found = [0; 2];
    let mut result = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if let Some((i, (key, value))) = bindings
            .iter()
            .enumerate()
            .find(|(_, (key, _))| line.starts_with(key))
        {
            found[i] += 1;
            result.push_str(key);
            result.push(' ');
            result.push_str(&serde_json::to_string(value).map_err(|e| e.to_string())?);
            if line.ends_with("\r\n") {
                result.push('\r');
            }
            if line.ends_with('\n') {
                result.push('\n');
            }
        } else {
            result.push_str(line);
        }
    }
    if found != [1, 1] {
        return Err(
            "expected exactly one repository and dev_branch in template-answers.yml".into(),
        );
    }
    Ok(result)
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

fn write_changed(path: &Path, previous: Option<&[u8]>, next: Option<&[u8]>) -> Result<(), String> {
    if previous == next {
        return Ok(());
    }
    match next {
        Some(bytes) => crate::shared::replace(path, bytes, 0o600),
        None => fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display())),
    }
}

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
