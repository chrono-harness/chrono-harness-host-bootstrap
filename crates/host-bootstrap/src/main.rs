//! 本宿主原生 bootstrap 入口；原操作、两流和所有者阶段均由实际生产者发布。
use vstd::contrib::exec_spec::ExecSpecEq;
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
#[derive(PartialEq,Eq)]
pub enum CargoCommand { Report, Original, Replay, Other }

/// Select the operation that owns Cargo evidence production or replay.
pub fn cargo_command(name:&str) -> (command:CargoCommand)
    ensures (command == CargoCommand::Report) == (name@=="collect-cargo-proof"@),
        (command == CargoCommand::Original) == (name@=="collect-cargo-proof-original"@),
        (command == CargoCommand::Replay) == (name@=="validate-cargo-proof-originals"@),
        (command == CargoCommand::Other) == (name@!="collect-cargo-proof"@ && name@!="collect-cargo-proof-original"@ && name@!="validate-cargo-proof-originals"@),
{
   if <&str>::exec_eq(name,"collect-cargo-proof") {CargoCommand::Report}
   else if <&str>::exec_eq(name,"collect-cargo-proof-original"){CargoCommand::Original}
   else if <&str>::exec_eq(name,"validate-cargo-proof-originals"){CargoCommand::Replay}
   else {CargoCommand::Other}
}
}
#[cfg(not(verus_keep_ghost))]
use chrono_host_bootstrap::{
    bindings,
    construction::{BuildFailure, Construction},
    download, lifecycle, owner, profile, proved_build, shared,
};
#[cfg(not(verus_keep_ghost))]
use serde_json::json;
#[cfg(not(verus_keep_ghost))]
use std::{io::Write, path::Path, process::ExitCode};

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
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
    if let Some((kind, output, sources)) =
        chrono_host_bootstrap::verified_cargo::owner_source_arguments(&arguments)
    {
        return proved_build::check_owner_sources(
            kind,
            Path::new(output),
            sources,
            chrono_host_bootstrap::compiler::compiler,
        )
        .map_err(|error| BuildFailure::from(format!("E_VERUS_COVERAGE: {error}")));
    }
    match arguments.as_slice() {
        [command] if command == "--version" => {
            println!("chrono-host-bootstrap {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [
            command,
            directory,
            operation,
            tool,
            workspace,
            packages @ ..,
        ] if command == "run-cargo-proofs" => {
            let root = std::env::current_dir().map_err(|e| BuildFailure::from(e.to_string()))?;
            chrono_host_bootstrap::cargo_proofs::Build::new(
                &root, directory, operation, tool, workspace,
            )
            .and_then(|build| build.run(packages, &environment))
            .map_err(|e| BuildFailure::from(format!("E_VERUS_COVERAGE: {e}")))
        }
        [
            command,
            directory,
            operation,
            tool,
            workspace,
            package,
            version,
            commit,
            manifest,
            sources @ ..,
        ] if matches!(
            cargo_command(command),
            CargoCommand::Report | CargoCommand::Original
        ) =>
        {
            let root = std::env::current_dir().map_err(|e| BuildFailure::from(e.to_string()))?;
            let contract = chrono_host_bootstrap::compiler::OutputContract {
                version: version.clone(),
                commit: commit.clone(),
            };
            let report = chrono_host_bootstrap::cargo_proofs::Build::new(
                &root, directory, operation, tool, workspace,
            )
            .and_then(|build| {
                if cargo_command(command) == CargoCommand::Original {
                    build.collect_original(package, &contract, manifest, sources)
                } else {
                    build.collect(package, &contract, manifest, sources)
                }
            })
            .map_err(|e| BuildFailure::from(format!("E_VERUS_COVERAGE: {e}")))?;
            std::io::stdout()
                .lock()
                .write_all(report.as_bytes())
                .map_err(|e| BuildFailure::from(e.to_string()))
        }
        [command] if cargo_command(command) == CargoCommand::Replay => {
            let archive: chrono_host_bootstrap::cargo_proofs::Replay =
                serde_json::from_reader(std::io::stdin().lock())
                    .map_err(|e| BuildFailure::from(e.to_string()))?;
            let roots = archive
                .validate(chrono_host_bootstrap::compiler::proof_rows)
                .map_err(|e| BuildFailure::from(format!("E_VERUS_COVERAGE: {e}")))?;
            serde_json::to_writer(std::io::stdout().lock(), &roots)
                .map_err(|e| BuildFailure::from(e.to_string()))
        }
        [command, directory, operation, verifier, source] if command == "run-compiler-proof" => {
            let root = std::env::current_dir().map_err(|e| BuildFailure::from(e.to_string()))?;
            chrono_host_bootstrap::owner_proof::Proof::new(
                &root,
                Path::new(directory),
                operation,
                verifier,
                source,
            )
            .and_then(|proof| proof.run(&environment))
            .map_err(|e| BuildFailure::from(format!("E_VERUS_COVERAGE: {e}")))
        }
        [
            command,
            directory,
            operation,
            verifier,
            version,
            commit,
            sources @ ..,
        ] if command == "collect-compiler-proof" => {
            let source = sources
                .first()
                .ok_or_else(|| BuildFailure::from("E_VERUS_COVERAGE: missing proof root"))?;
            let root = std::env::current_dir().map_err(|e| BuildFailure::from(e.to_string()))?;
            let contract = chrono_host_bootstrap::compiler::OutputContract {
                version: version.clone(),
                commit: commit.clone(),
            };
            let original = chrono_host_bootstrap::owner_proof::Proof::new(
                &root,
                Path::new(directory),
                operation,
                verifier,
                source,
            )
            .and_then(|proof| {
                proof.collect(
                    &contract,
                    sources,
                    chrono_host_bootstrap::compiler::compiler,
                )
            })
            .map_err(|e| BuildFailure::from(format!("E_VERUS_COVERAGE: {e}")))?;
            std::io::stdout()
                .lock()
                .write_all(original.as_bytes())
                .map_err(|e| BuildFailure::from(e.to_string()))
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

#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let [command, rest @ ..] = arguments.as_slice()
        && command == "native-scratch"
    {
        let result = (|| -> std::io::Result<u8> {
            let root = std::env::current_dir()?.canonicalize()?;
            let git = std::ffi::OsStr::new("git");
            match rest {
                [action, program, args @ ..] if action == "run" => {
                    chrono_host_bootstrap::native_scratch::run(&root, program, args, git)
                }
                [action, seconds] if action == "maintain" => {
                    let seconds = seconds
                        .to_str()
                        .and_then(|v| v.parse().ok())
                        .ok_or_else(|| std::io::Error::other("invalid retention seconds"))?;
                    let removed =
                        chrono_host_bootstrap::native_scratch::maintain(&root, seconds, git)?;
                    println!("native check directories removed: {removed}");
                    Ok(0)
                }
                _ => Err(std::io::Error::other(
                    "usage: native-scratch run PROGRAM [ARG ...] | maintain SECONDS",
                )),
            }
        })();
        return match result {
            Ok(code) => code.into(),
            Err(error) => {
                eprintln!("native scratch: {error}");
                2.into()
            }
        };
    }
    if let [command, selection] = arguments.as_slice()
        && command == "ci-install-table-v1"
    {
        let check_storage_and_links = match selection.to_str() {
            Some("manager") => false,
            Some("manager-storage-links") => true,
            _ => {
                eprintln!("expected manager or manager-storage-links");
                return ExitCode::from(2);
            }
        };
        print!(
            "{}",
            chrono_host_bootstrap::template::ci_install_table(check_storage_and_links)
        );
        return ExitCode::SUCCESS;
    }
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
