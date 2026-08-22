use std::process::ExitCode;

fn main() -> ExitCode {
    match irodori_samples_tool::cli::run_cli(std::env::args().skip(1)) {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}
