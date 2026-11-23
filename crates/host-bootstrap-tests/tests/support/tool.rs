//! Rust toolchain command fixture for the bootstrap binding tests.
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let invocation = env::args().next().expect("missing fixture invocation");
    let executable = Path::new(&invocation);
    let name = executable.file_name().and_then(|value| value.to_str());
    let arguments: Vec<_> = env::args().skip(1).collect();
    let args: Vec<_> = arguments.iter().map(String::as_str).collect();

    match (name, args.as_slice()) {
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
