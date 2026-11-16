use std::process::ExitCode;

fn main() -> ExitCode {
    match chromasync_cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Same report `fn main() -> anyhow::Result<()>` would print.
            eprintln!("Error: {error:?}");
            ExitCode::from(chromasync_cli::exit_code(&error))
        }
    }
}
