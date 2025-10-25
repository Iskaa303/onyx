use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    ChatGPT,
    Claude,
    Ollama,
    LlamaCpp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGPTConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    pub api_key: String,
    pub endpoint: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub endpoint: String,
    pub model: String,
    pub temperature: f32,
    pub num_predict: u32,
    pub top_k: u32,
    pub top_p: f32,
    pub repeat_penalty: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppConfig {
    pub endpoint: String,
    pub temperature: f32,
    pub n_predict: u32,
    pub top_k: u32,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub repeat_last_n: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderConfigs {
    pub chatgpt: ChatGPTConfig,
    pub claude: ClaudeConfig,
    pub ollama: OllamaConfig,
    pub llamacpp: LlamaCppConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub active_provider: Provider,
    pub providers: ProviderConfigs,
}

impl Default for ChatGPTConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 2048,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        }
    }
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: "https://api.anthropic.com/v1/messages".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 4096,
            temperature: 1.0,
            top_p: 0.9,
            top_k: 40,
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434/api/generate".to_string(),
            model: "llama3.2".to_string(),
            temperature: 0.7,
            num_predict: 2048,
            top_k: 40,
            top_p: 0.9,
            repeat_penalty: 1.1,
        }
    }
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080/completion".to_string(),
            temperature: 0.7,
            n_predict: 512,
            top_k: 40,
            top_p: 0.9,
            repeat_penalty: 1.1,
            repeat_last_n: 64,
        }
    }
}

impl Default for ProviderConfigs {
    fn default() -> Self {
        Self {
            chatgpt: ChatGPTConfig::default(),
            claude: ClaudeConfig::default(),
            ollama: OllamaConfig::default(),
            llamacpp: LlamaCppConfig::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            active_provider: Provider::Claude,
            providers: ProviderConfigs::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let settings_path = Self::config_path()?;

        if !settings_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&settings_path)
            .context("Failed to read settings file")?;

        let settings: Settings = toml::from_str(&content)
            .context("Failed to parse settings file")?;

        Ok(settings)
    }

    pub fn save(&self) -> Result<()> {
        let settings_dir = Self::config_dir()?;
        fs::create_dir_all(&settings_dir)
            .context("Failed to create settings directory")?;

        let settings_path = Self::config_path()?;
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize settings")?;

        fs::write(&settings_path, content)
            .context("Failed to write settings file")?;

        Ok(())
    }

    pub fn init() -> Result<()> {
        let settings_dir = Self::config_dir()?;

        fs::create_dir_all(&settings_dir)
            .context("Failed to create .onyx directory")?;

        let settings = Self::default();
        settings.save()?;

        Ok(())
    }

    fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .context("Failed to determine home directory")?;
        Ok(home.join(".onyx"))
    }

    fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("settings.toml"))
    }
}
