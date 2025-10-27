use anyhow::Result;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::{anthropic, ollama, openai};

use crate::core::{Config, Message, Provider};

pub enum ChatAgent {
    OpenAI(Agent<openai::responses_api::ResponsesCompletionModel>),
    Anthropic(Agent<anthropic::completion::CompletionModel>),
    Ollama(Agent<ollama::CompletionModel<reqwest::Client>>),
}

impl ChatAgent {
    pub async fn new(config: &Config) -> Result<Self> {
        config.validate()?;

        let provider_config = config.get_active_provider();

        match config.active_provider {
            Provider::OpenAI => {
                let api_key = provider_config.api_key.as_ref().unwrap();
                let client = openai::Client::new(api_key);
                let agent = client.agent(&provider_config.model).build();
                Ok(Self::OpenAI(agent))
            }
            Provider::Anthropic => {
                let api_key = provider_config.api_key.as_ref().unwrap();
                let client = anthropic::Client::new(api_key);
                let agent = client.agent(&provider_config.model).build();
                Ok(Self::Anthropic(agent))
            }
            Provider::Ollama => {
                let client = ollama::Client::new();
                let agent = client.agent(&provider_config.model).build();
                Ok(Self::Ollama(agent))
            }
        }
    }

    pub async fn send(&self, message: Message) -> Result<Message> {
        let response = match self {
            Self::OpenAI(agent) => agent.prompt(&message.content).await?,
            Self::Anthropic(agent) => agent.prompt(&message.content).await?,
            Self::Ollama(agent) => agent.prompt(&message.content).await?,
        };
        Ok(Message::assistant(response))
    }
}
