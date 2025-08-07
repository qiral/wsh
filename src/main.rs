mod completion;
mod config;
mod shell;
mod utils;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "rsh")]
#[command(about = "A modern shell written in Rust")]
#[command(version)]
struct Cli {
    #[arg(short = 'f', long)]
    config: Option<std::path::PathBuf>,

    #[arg(short, long)]
    interactive: bool,

    #[arg(short = 'c', long)]
    command: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::init();

    let config = config::Config::load(cli.config.as_deref())?;
    let mut shell = shell::Shell::new(config)?;

    if let Some(cmd) = cli.command {
        shell.execute_command(&cmd)
    } else {
        shell.run_interactive()
    }
}
