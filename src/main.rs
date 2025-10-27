mod agent;
mod core;
mod interface;

use anyhow::Result;

use crate::agent::ChatAgent;
use crate::core::{Config, Message};
use crate::interface::App;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::load()?;

    let mut terminal = ratatui::init();
    let mut app = App::new();

    let agent = match ChatAgent::new(&config).await {
        Ok(agent) => Some(agent),
        Err(e) => {
            let provider_config = config.get_active_provider();
            let needs_api_key = provider_config.api_key.is_none()
                || provider_config.api_key.as_ref().unwrap().is_empty();

            if needs_api_key {
                app.add_message(Message::assistant(
                    "Welcome to Onyx!\n\n\
                    No API key found for the active provider.\n\
                    1. Type /config to see the config file location\n\
                    2. Edit the file and add your API key\n\
                    3. Restart the application\n\n\
                    You can still use commands like /help and /config."
                        .to_string(),
                ));
                None
            } else {
                ratatui::restore();
                return Err(e);
            }
        }
    };

    loop {
        terminal.draw(|frame| {
            app.draw(frame);
        })?;

        app.handle_event()?;

        if app.should_quit() {
            break;
        }

        if let Some(input) = app.take_input() {
            if let Some(cmd_response) = app.handle_command(&input) {
                app.add_message(Message::assistant(cmd_response));
            } else if let Some(ref agent) = agent {
                let user_msg = Message::user(input);
                app.add_message(user_msg.clone());

                let response = agent.send(user_msg).await?;
                app.add_message(response);
            } else {
                app.add_message(Message::assistant(
                    "Please configure your API key first. Type /config for instructions."
                        .to_string(),
                ));
            }
        }
    }

    ratatui::restore();
    Ok(())
}
