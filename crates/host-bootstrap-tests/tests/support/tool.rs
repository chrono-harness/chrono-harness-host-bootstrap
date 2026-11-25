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
