use std::io;

mod app;
mod cli;
mod constants;
mod domain;
mod sand;
mod storage;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        cli::run_cli();
        return Ok(());
    }

    app::run_ui()
}
