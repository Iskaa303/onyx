mod fs;
mod display;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "onyx")]
#[command(about = "Simple CLI tool to organize info using AI", long_about = None)]
struct Cli {
    #[arg(value_name = "PATH", help = "Path to the directory to read (defaults to current directory)")]
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let target_path = cli.path.unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to get current directory")
    });

    let absolute_path = target_path.canonicalize()?;

    let entries = fs::read_directory(&absolute_path)?;
    display::show_entries(&entries, &absolute_path);
    Ok(())
}
