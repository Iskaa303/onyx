use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
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
#[serde(default)]
pub struct Config {
    pub active_provider: Provider,
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
    pub ollama: ProviderConfig,
    pub qdrant_url: String,
    pub qdrant_api_key: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_provider: Provider::OpenAI,
            openai: ProviderConfig {
                api_key: None,
                model: "gpt-4".to_string(),
                url: None,
            },
            anthropic: ProviderConfig {
                api_key: None,
                model: "claude-3-5-sonnet-20241022".to_string(),
                url: None,
            },
            ollama: ProviderConfig {
                api_key: None,
                model: "llama3.2".to_string(),
                url: Some("http://localhost:11434".to_string()),
            },
            qdrant_url: "http://localhost:6334".to_string(),
            qdrant_api_key: None,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            eprintln!("Created default config at: {}", path.display());
            eprintln!("Please edit it to add your API keys.");
            return Ok(config);
        }

        let content = fs::read_to_string(&path).context("Failed to read config file")?;

        match serde_json::from_str::<Config>(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                eprintln!("Warning: Config file is corrupted or outdated.");
                eprintln!("Error: {}", e);

                let backup_path = Self::backup_path()?;
                fs::copy(&path, &backup_path).context("Failed to backup old config")?;
                eprintln!("Backed up old config to: {}", backup_path.display());

                let config = Self::default();
                config.save()?;
                eprintln!("Created new default config at: {}", path.display());

                Ok(config)
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir).context("Failed to create config directory")?;

        let path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&path, content).context("Failed to write config file")?;

        Ok(())
    }

    pub fn get_active_provider(&self) -> &ProviderConfig {
        match self.active_provider {
            Provider::OpenAI => &self.openai,
            Provider::Anthropic => &self.anthropic,
            Provider::Ollama => &self.ollama,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let provider = self.get_active_provider();
        let provider_name = match self.active_provider {
            Provider::OpenAI => "OpenAI",
            Provider::Anthropic => "Anthropic",
            Provider::Ollama => "Ollama",
        };

        if let Provider::Ollama = self.active_provider {
            return Ok(());
        }

        if provider.api_key.is_none() || provider.api_key.as_ref().unwrap().is_empty() {
            anyhow::bail!(
                "{} API key not configured.\n\
                Please edit {} and add your API key for {}.",
                provider_name,
                Self::config_path_display(),
                provider_name
            );
        }

        Ok(())
    }

    fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to determine home directory")?;
        Ok(home.join(".onyx"))
    }

    fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }

    fn backup_path() -> Result<PathBuf> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(Self::config_dir()?.join(format!("config.json.backup.{}", timestamp)))
    }

    pub fn config_path_display() -> String {
        Self::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "~/.onyx/config.json".to_string())
    }
}
