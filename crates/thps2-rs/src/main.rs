use std::process::ExitCode;

fn main() -> ExitCode {
    match thps2_rs::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("[error] {error}");
            ExitCode::FAILURE
        }
    }
}
