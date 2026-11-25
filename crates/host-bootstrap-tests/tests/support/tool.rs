//! Rust toolchain command fixture for the bootstrap binding tests.
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let invocation = env::args().next().expect("missing fixture invocation");
    let executable = Path::new(&invocation);
    let name = executable.file_name().and_then(|value| value.to_str());
    let arguments: Vec<_> = env::args().skip(1).collect();
    let args: Vec<_> = arguments.iter().map(String::as_str).collect();

    match (name, args.as_slice()) {
        (
            Some("git"),
            [
                "ls-files",
                "--cached",
                "--with-tree=HEAD",
                "-z",
                "--",
                "target/native-checks",
            ],
        ) => {
            use std::io::Write;
            std::io::stdout()
                .write_all(&std::fs::read("tracked").unwrap_or_default())
                .unwrap();
        }
        (Some("fixture-compiler-query"), [case]) => {
            let input = if case.starts_with("closed-") {
                serde_json::json!("x".repeat(8 * 1024 * 1024))
            } else {
                serde_json::json!({"source": "λ"})
            };
            let result = chrono_host_bootstrap::compiler::compiler::<bool>(input);
            println!("{}", serde_json::to_string(&result).unwrap());
        }
        (Some("chrono-harness"), ["compiler-output-v1"]) => {
            use std::io::{Read, Write};
            let case = env::var("COMPILER_FIXTURE_CASE").unwrap();
            if !case.starts_with("closed-") {
                let mut request = Vec::new();
                std::io::stdin().read_to_end(&mut request).unwrap();
                std::fs::write("request", request).unwrap();
            }
            let (status, stdout, stderr): (u8, &[u8], &[u8]) = match case.as_str() {
                "ok" | "closed-ok" => (0, b"true", b"ignored"),
                "bad-json" | "closed-bad-json" => (0, b"{", b"ignored"),
                "diagnostic" | "closed-diagnostic" => (17, b"true", "diagnostic λ\n\n".as_bytes()),
                "closed-status" | "status" => (17, b"{", b""),
                "closed-invalid-utf8" | "invalid-utf8" => (17, b"true", &[0xff]),
                "newlines" => (17, b"true", b"\n\n"),
                _ => return ExitCode::from(99),
            };
            std::io::stdout().write_all(stdout).unwrap();
            std::io::stderr().write_all(stderr).unwrap();
            std::fs::write("completed", std::process::id().to_string()).unwrap();
            return ExitCode::from(status);
        }
        (Some("fixture-native"), [outcome]) => {
            let scratch = std::env::var_os("CHRONO_NATIVE_SCRATCH").expect("missing scratch");
            std::fs::write(Path::new(&scratch).join("object"), b"compiled").unwrap();
            return ExitCode::from(outcome.parse::<u8>().expect("invalid fixture exit"));
        }
        (Some("rustup"), ["which", "--toolchain", "1.95.0", tool @ ("cargo" | "rustc")]) => {
            println!("{}", executable.with_file_name(tool).display());
        }
        (Some(tool @ ("cargo" | "rustc")), ["--version"])
            if env::var("RUSTUP_TOOLCHAIN").as_deref() == Ok("1.95.0") =>
        {
            println!("{tool} 1.95.0 (fixture)");
        }
        _ => return ExitCode::from(99),
    }
    ExitCode::SUCCESS
}
