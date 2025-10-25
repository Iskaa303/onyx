mod fs;
mod display;
mod settings;
mod ai;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "onyx")]
#[command(about = "Simple CLI tool to organize info using AI", long_about = None)]
struct Cli {
    #[arg(value_name = "PATH", help = "Path to the directory to read (defaults to current directory)")]
    path: Option<PathBuf>,

    #[arg(long, help = "Initialize settings in ~/.onyx")]
    init: bool,

    #[arg(short, long, help = "Ask AI a question")]
    ask: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.init {
        settings::Settings::init()?;
        println!("Settings initialized in ~/.onyx/settings.toml");
        return Ok(());
    }

    if let Some(question) = cli.ask {
        let settings = settings::Settings::load()?;
        let response = ai::send_message(&settings, &question)?;
        println!("\n{}", response);
        return Ok(());
    }

    let target_path = cli.path.unwrap_or_else(|| {
        std::env::current_dir().expect("Failed to get current directory")
    });

    let absolute_path = target_path.canonicalize()?;

    let entries = fs::read_directory(&absolute_path)?;
    display::show_entries(&entries, &absolute_path);
    Ok(())
}
