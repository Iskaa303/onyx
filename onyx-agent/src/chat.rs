use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::{anthropic, ollama, openai};
use thiserror::Error;
use tokio::sync::mpsc;

use onyx_core::{Config, Message, Provider};

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    ConfigError(#[from] onyx_core::ConfigError),

    #[error("Agent error: {0}")]
    RigError(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;

#[derive(Debug, Clone)]
pub enum StreamEvent {
    ThinkingStart,
    ThinkingChunk(String),
    ThinkingEnd,
    ContentChunk(String),
    Done,
    Error(String),
}

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
            Self::OpenAI(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
            Self::Anthropic(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
            Self::Ollama(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
        };
        Ok(Message::assistant(response))
    }

    pub async fn send_stream(
        &self,
        message: Message,
        tx: mpsc::UnboundedSender<StreamEvent>,
    ) -> Result<()> {
        let response_text = match self {
            Self::OpenAI(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
            Self::Anthropic(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
            Self::Ollama(agent) => agent
                .prompt(&message.content)
                .await
                .map_err(|e| AgentError::RigError(e.to_string()))?,
        };

        let mut in_thinking = false;
        let mut current_chunk = String::new();

        for c in response_text.chars() {
            current_chunk.push(c);

            if current_chunk.ends_with("<thinking>") {
                in_thinking = true;
                current_chunk.clear();
                let _ = tx.send(StreamEvent::ThinkingStart);
            } else if current_chunk.ends_with("</thinking>") && in_thinking {
                let thinking_text =
                    current_chunk.strip_suffix("</thinking>").unwrap_or(&current_chunk).to_string();
                if !thinking_text.is_empty() {
                    let _ = tx.send(StreamEvent::ThinkingChunk(thinking_text));
                }
                let _ = tx.send(StreamEvent::ThinkingEnd);
                in_thinking = false;
                current_chunk.clear();
            } else if current_chunk.len() >= 5 {
                let to_send = current_chunk.clone();
                current_chunk.clear();

                if in_thinking {
                    if tx.send(StreamEvent::ThinkingChunk(to_send)).is_err() {
                        break;
                    }
                } else if tx.send(StreamEvent::ContentChunk(to_send)).is_err() {
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }

        if !current_chunk.is_empty() {
            if in_thinking {
                let _ = tx.send(StreamEvent::ThinkingChunk(current_chunk));
                let _ = tx.send(StreamEvent::ThinkingEnd);
            } else {
                let _ = tx.send(StreamEvent::ContentChunk(current_chunk));
            }
        }

        let _ = tx.send(StreamEvent::Done);
        Ok(())
    }
}
