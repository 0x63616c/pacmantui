use std::process::ExitCode;

fn main() -> ExitCode {
    match pacmantui::app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("pacmantui: {e}");
            ExitCode::FAILURE
        }
    }
}
