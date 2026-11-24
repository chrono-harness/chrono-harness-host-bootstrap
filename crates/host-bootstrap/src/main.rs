//! 本宿主原生 bootstrap 入口；原操作、两流和所有者阶段均由实际生产者发布。
use chrono_host_bootstrap::{
    bindings,
    construction::{BuildFailure, Construction},
    download, lifecycle, owner, profile, proved_build, shared,
};
use serde_json::json;
use std::{io::Write, path::Path, process::ExitCode};

fn bootstrap(host: &str, config: &str, stage_only: bool) -> Result<(), BuildFailure> {
    let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
    let environment = std::env::vars_os().collect();
    let native = bindings::native_platform()?;
    let loaded = profile::Loaded::read(&root, config, &native, &environment)?;
    if stage_only {
        owner::require_capabilities(&root, &environment)?;
        if loaded.profile.owner_stage.is_none() {
            return Err("owner stage declaration is missing".into());
        }
    } else if lifecycle::participate(&root, config, &loaded.profile, &native, &environment)? {
        return Ok(());
    }
    let previous = loaded
        .profile
        .owner_stage
        .as_ref()
        .map(|stage| {
            owner::require_capabilities(&root, &environment)?;
            owner::file_binding(&profile::path(
                &root,
                &stage.selected,
                b".chrono-harness/bin/",
            )?)
        })
        .transpose()?;
    let stage = previous.is_some();
    let mut construction = Construction::begin(&root, loaded, &environment)?;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let outcome = (|| {
        construction.prerequisites(&mut stdout, &mut stderr)?;
        construction.select()?;
        construction.operations_reusing_owner(&native, &mut stdout, &mut stderr)?;
        let installations: Vec<_> = construction
            .profile
            .install
            .iter()
            .map(|item| (item.source.clone(), item.destination.clone()))
            .collect();
        for (source, destination) in installations {
            construction.install_tool(&root.join(source), &destination)?;
        }
        if let Some(previous) = previous {
            construction.stage_owner(&previous)?;
        }
        Ok(())
    })();
    let published = construction.finish(outcome)?;
    eprintln!(
        "bootstrap original report: {}",
        published.original.display()
    );
    published.outcome?;
    if stage {
        let reference = published
            .original
            .strip_prefix(&root)
            .map_err(|error| error.to_string())?
            .to_str()
            .ok_or("non UTF-8 owner original report")?;
        let digest = chrono_host_bootstrap::files::file_identity(&published.original)?.0;
        println!(
            "chrono-owner-stage-result {}",
            json!({"path":reference,"sha256":digest})
        );
    }
    Ok(())
}

fn run() -> Result<(), BuildFailure> {
    let arguments: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|value| {
            value
                .into_string()
                .map_err(|_| BuildFailure::from("non UTF-8 bootstrap argument"))
        })
        .collect::<Result<_, _>>()?;
    let environment = std::env::vars_os().collect();
    let native = bindings::native_platform()?;
    match arguments.as_slice() {
        [command] if command == "--version" => {
            println!("chrono-host-bootstrap {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [command, version, commit, report, output, sources @ ..]
            if command == "check-compiler-proof" =>
        {
            let contract = chrono_host_bootstrap::compiler::OutputContract {
                version: version.clone(),
                commit: commit.clone(),
            };
            let original = proved_build::check_owner_proof(
                &contract,
                Path::new(report),
                Path::new(output),
                sources,
                chrono_host_bootstrap::compiler::compiler,
            )
            .map_err(|error| BuildFailure::from(format!("E_VERUS_COVERAGE: {error}")))?;
            std::io::stdout()
                .lock()
                .write_all(original.as_bytes())
                .map_err(|error| BuildFailure::from(error.to_string()))
        }
        [command, kind, output, sources @ ..] if command == "check-compiler-sources" => {
            proved_build::check_owner_sources(
                kind,
                Path::new(output),
                sources,
                chrono_host_bootstrap::compiler::compiler,
            )
            .map_err(|error| BuildFailure::from(format!("E_VERUS_COVERAGE: {error}")))
        }
        [command, host, config] if command == "compile" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            proved_build::build(
                &root,
                config,
                &environment,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
            )
        }
        [command, host, checksum, curl, headers, url] if command == "download" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            let archive =
                download::prepare(&root, checksum, Path::new(curl), headers, url, &environment)?;
            println!(
                "{}",
                serde_json::to_string(&archive).map_err(|error| error.to_string())?
            );
            Ok(())
        }
        [host, config, command] if command == "produce" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            shared::Shared::read(&root, config, &native)?.produce(&root, &environment, &native)
        }
        [host, config, command, selected] if command == "install" || command == "verify" => {
            let root = std::fs::canonicalize(Path::new(host)).map_err(|error| error.to_string())?;
            let shared = shared::Shared::read(&root, config, &native)?;
            if command == "install" {
                shared.install(&root, &environment, &native, selected)
            } else {
                shared.verify(&root, &environment, &native, selected)
            }
        }
        [command, host, config] if command == "construct-owner-stage" => {
            bootstrap(host, config, true)
        }
        [host, config] => bootstrap(host, config, false),
        _ => Err("unknown native bootstrap command or arguments".into()),
    }
}

fn finish(result: Result<(), BuildFailure>) -> u8 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("E_BOOTSTRAP: {}", error.message);
            error
                .exit_code
                .and_then(|code| u8::try_from(code).ok())
                .filter(|code| *code != 0)
                .unwrap_or(1)
        }
    }
}
fn logged(root: &Path, operation: &str) -> Result<u8, String> {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    match root.join(".chrono-harness/logging.json").symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(finish(run())),
        Err(e) => return Err(e.to_string()),
        Ok(_) => {}
    }
    let mut child = Command::new("chrono-diagnostics")
        .arg("cli-events-v1")
        .arg(root)
        .args(["chrono-host-bootstrap", operation])
        .arg(std::process::id().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("chrono-diagnostics cli-events-v1: {e}"))?;
    let mut writer = child.stdin.take().expect("piped stdin");
    let mut reader = BufReader::new(child.stdout.take().expect("piped stdout"));
    let result = (|| -> Result<u8, String> {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if serde_json::from_str::<serde_json::Value>(&line).ok() != Some(json!({"ready":true})) {
            return Err("chrono-diagnostics did not open the CLI log".into());
        }
        let result = run();
        if let Err(error) = &result {
            serde_json::to_writer(
                &mut writer,
                &json!({"diagnostic":format!("E_BOOTSTRAP: {}",error.message)}),
            )
            .map_err(|e| e.to_string())?;
            writer.write_all(b"\n").map_err(|e| e.to_string())?;
        }
        let status = finish(result);
        serde_json::to_writer(&mut writer, &json!({"exit_code":status}))
            .map_err(|e| e.to_string())?;
        writer
            .write_all(b"\n")
            .and_then(|()| writer.flush())
            .map_err(|e| e.to_string())?;
        Ok(status)
    })();
    drop(writer);
    let status = child.wait().map_err(|e| e.to_string())?;
    let wanted = result?;
    if status.code() != Some(i32::from(wanted)) {
        return Err(format!(
            "chrono-diagnostics cli-events-v1 exited {status} after {wanted}"
        ));
    }
    Ok(wanted)
}
fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if matches!(arguments.as_slice(), [arg] if arg == "prepare-template") {
        return match chrono_host_bootstrap::template::run() {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("E_BOOTSTRAP_TEMPLATE: {error}");
                2.into()
            }
        };
    }
    if arguments.first().and_then(|arg| arg.to_str()) == Some("original-report-v1") {
        use std::io::Write;
        let read = || -> Result<(), String> {
            if arguments.len() != 1 {
                return Err("original-report-v1 reads its query from stdin".into());
            }
            let root = std::env::current_dir().map_err(|e| e.to_string())?;
            let reply = chrono_host_bootstrap::report_query::read(
                &root,
                &std::env::vars_os().collect(),
                std::io::stdin().lock(),
            )?;
            let mut out = std::io::stdout().lock();
            serde_json::to_writer(&mut out, &reply).map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())
        };
        return match read() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("E_BOOTSTRAP_ORIGINAL: {e}");
                2.into()
            }
        };
    }
    let (host, operation) = match arguments.as_slice() {
        [host, _, operation] if operation.to_str() == Some("produce") => (Some(host), "produce"),
        [host, _, operation, _] if matches!(operation.to_str(), Some("install" | "verify")) => {
            (Some(host), operation.to_str().unwrap())
        }
        [operation, host, _]
            if matches!(
                operation.to_str(),
                Some("construct-owner-stage" | "compile")
            ) =>
        {
            (Some(host), operation.to_str().unwrap())
        }
        [host, _] => (Some(host), "bootstrap"),
        _ => (None, ""),
    };
    let default_root = arguments
        .windows(2)
        .find(|pair| pair[0] == "--host-root")
        .map(|pair| Path::new(&pair[1]))
        .unwrap_or(Path::new("."));
    let (root, operation) = match host {
        Some(host) => (Path::new(host), operation),
        None => (
            default_root,
            arguments.first().and_then(|arg| arg.to_str()).unwrap_or(""),
        ),
    };
    match logged(root, operation) {
        Ok(code) => code.into(),
        Err(error) => {
            eprintln!("{error}");
            2.into()
        }
    }
}
