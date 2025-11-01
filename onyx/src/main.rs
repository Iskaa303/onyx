use eyre::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use onyx_agent::ChatAgent;
use onyx_core::{Config, ConfigSchema, Message};
use onyx_tui::App;

enum AppEvent {
    Response(Message),
}

fn parse_args() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        return None;
    }

    match args[0].as_str() {
        "-c" | "--config" => {
            if args.len() < 2 {
                eprintln!("Error: --config requires a path argument");
                std::process::exit(1);
            }
            Some(PathBuf::from(&args[1]))
        }
        "-h" | "--help" => {
            println!("Onyx - AI Chat Terminal Application");
            println!();
            println!("USAGE:");
            println!("    onyx [OPTIONS]");
            println!();
            println!("OPTIONS:");
            println!("    -c, --config <PATH>    Specify custom config file path");
            println!("    -h, --help             Print this help message");
            println!();
            println!("EXAMPLES:");
            println!(
                "    onyx                              # Use default config (~/.onyx/config.json)"
            );
            println!("    onyx --config /path/to/config.json");
            std::process::exit(0);
        }
        _ => {
            eprintln!("Error: Unknown argument '{}'", args[0]);
            eprintln!("Use --help for usage information");
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let custom_config_path = parse_args();
    let config = Config::load_from(custom_config_path)?;

    let mut terminal = ratatui::init();
    let mut app = App::new(config.clone());

    let agent = match ChatAgent::new(&config).await {
        Ok(agent) => Some(Arc::new(agent)),
        Err(e) => {
            let provider_config = config.get_active_provider();
            let needs_api_key = provider_config.api_key.is_none()
                || provider_config.api_key.as_ref().unwrap().is_empty();

            if needs_api_key {
                app.add_message(Message::assistant(
                    "Welcome to Onyx!\n\n\
                    No API key found for the active provider.\n\
                    Type /config to open the configuration editor and set up your API keys.\n\n\
                    You can still use commands like /help and /config."
                        .to_string(),
                ));
                None
            } else {
                ratatui::restore();
                return Err(e.into());
            }
        }
    };

    let (tx, mut rx) = mpsc::unbounded_channel();

    loop {
        terminal.draw(|frame| {
            app.draw(frame);
        })?;

        app.handle_event()?;

        if app.should_quit() {
            break;
        }

        if let Some(input) = app.take_input() {
            if input.starts_with('/') {
                if let Some(cmd_response) = app.handle_command(&input) {
                    app.add_message(Message::assistant(cmd_response));
                }
            } else {
                let user_msg = Message::user(input.clone());
                app.add_message(user_msg.clone());

                if let Some(ref agent) = agent {
                    app.set_processing(true);

                    let agent_arc = Arc::clone(agent);
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        match agent_arc.send(user_msg).await {
                            Ok(response) => {
                                let _ = tx_clone.send(AppEvent::Response(response));
                            }
                            Err(e) => {
                                let _ = tx_clone.send(AppEvent::Response(Message::assistant(
                                    format!("Error: {}", e),
                                )));
                            }
                        }
                    });
                } else {
                    app.add_message(Message::assistant(
                        "Please configure your API key first. Type /config to open the configuration editor."
                            .to_string(),
                    ));
                }
            }
        }

        while let Ok(AppEvent::Response(msg)) = rx.try_recv() {
            app.add_message(msg);
            app.set_processing(false);
        }
    }

    ratatui::restore();
    Ok(())
}
