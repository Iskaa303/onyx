use crate::config::*;
use crate::{config_defaults, config_fields};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Default, Display, EnumString, EnumIter,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Provider {
    #[default]
    #[strum(serialize = "OpenAI")]
    OpenAI,
    #[strum(serialize = "Anthropic")]
    Anthropic,
    #[strum(serialize = "Ollama")]
    Ollama,
}

#[derive(Debug, Clone, Serialize, Default, Deserialize)]
#[serde(default)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub active_provider: Provider,
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub ollama: ProviderConfig,
    pub qdrant_url: String,
    pub qdrant_api_key: Option<String>,
    pub timestamp_format: String,
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

config_defaults! {
    active_provider => Provider::OpenAI,
    openai => ProviderConfig {
        api_key: None,
        model: "gpt-4".to_string(),
        url: None,
    },
    anthropic => ProviderConfig {
        api_key: None,
        model: "claude-3-5-sonnet-20241022".to_string(),
        url: None,
    },
    ollama => ProviderConfig {
        api_key: None,
        model: "llama3.2".to_string(),
        url: Some("http://localhost:11434".to_string()),
    },
    qdrant_url => "http://localhost:6334".to_string(),
    qdrant_api_key => None,
    timestamp_format => "%Y-%m-%d %H:%M:%S".to_string(),
    config_path => None,
}

config_fields! {
    ["General"] => {
        active_provider: Enum(
            "Active Provider",
            "Select which AI provider to use",
            active_provider,
            Provider::iter().map(|p| p.to_string()).collect()
        )
    }

    ["OpenAI"] => {
        openai_api_key: OptionalString("API Key", "Required", openai.api_key),
        openai_model: String("Model", "e.g., gpt-4, gpt-3.5-turbo", openai.model),
        openai_url: OptionalString("URL", "Optional (leave empty for default)", openai.url)
    }

    ["Anthropic"] => {
        anthropic_api_key: OptionalString("API Key", "Required", anthropic.api_key),
        anthropic_model: String("Model", "e.g., claude-3-5-sonnet-20241022", anthropic.model),
        anthropic_url: OptionalString("URL", "Optional (leave empty for default)", anthropic.url)
    }

    ["Ollama"] => {
        ollama_api_key: OptionalString("API Key", "Not required for Ollama", ollama.api_key),
        ollama_model: String("Model", "e.g., llama3.2, mistral", ollama.model),
        ollama_url: OptionalString("URL", "Optional (leave empty for default)", ollama.url)
    }

    ["Qdrant"] => {
        qdrant_url: String("Qdrant URL", "Vector database URL", qdrant_url),
        qdrant_api_key: OptionalString("Qdrant API Key", "Optional Qdrant API key", qdrant_api_key)
    }

    ["Display"] => {
        timestamp_format: String(
            "Timestamp Format",
            "strftime format (e.g., %Y-%m-%d %H:%M:%S)",
            timestamp_format
        )
    }
}

impl Config {
    pub fn get_active_provider(&self) -> &ProviderConfig {
        match self.active_provider {
            Provider::OpenAI => &self.openai,
            Provider::Anthropic => &self.anthropic,
            Provider::Ollama => &self.ollama,
        }
    }

    pub fn validate(&self) -> ConfigResult<()> {
        let provider = self.get_active_provider();
        let provider_name = self.active_provider.to_string();

        if let Provider::Ollama = self.active_provider {
            return Ok(());
        }

        if provider.api_key.is_none() || provider.api_key.as_ref().unwrap().is_empty() {
            return Err(ConfigError::MissingApiKey(provider_name, Self::config_path_display()));
        }

        Ok(())
    }

    pub fn format_timestamp(&self, timestamp: std::time::SystemTime) -> String {
        use chrono::{DateTime, Local};
        let datetime: DateTime<Local> = timestamp.into();
        datetime.format(&self.timestamp_format).to_string()
    }
}
