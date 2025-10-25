use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use crate::settings::{Settings, Provider};

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
    top_p: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
}

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    temperature: f32,
    top_p: f32,
    top_k: u32,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
    top_k: u32,
    top_p: f32,
    repeat_penalty: f32,
}

#[derive(Serialize)]
struct LlamaCppRequest {
    prompt: String,
    temperature: f32,
    n_predict: u32,
    top_k: u32,
    top_p: f32,
    repeat_penalty: f32,
    repeat_last_n: u32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

pub fn send_message(settings: &Settings, prompt: &str) -> Result<String> {
    match settings.active_provider {
        Provider::ChatGPT => send_chatgpt(settings, prompt),
        Provider::Claude => send_claude(settings, prompt),
        Provider::Ollama => send_ollama(settings, prompt),
        Provider::LlamaCpp => send_llamacpp(settings, prompt),
    }
}

fn send_chatgpt(settings: &Settings, prompt: &str) -> Result<String> {
    let config = &settings.providers.chatgpt;

    if config.api_key.is_empty() {
        anyhow::bail!("ChatGPT api_key is empty. Please configure in ~/.onyx/settings.toml");
    }

    let request = OpenAIRequest {
        model: config.model.clone(),
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        top_p: config.top_p,
        frequency_penalty: config.frequency_penalty,
        presence_penalty: config.presence_penalty,
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .context("Failed to send request to ChatGPT")?;

    let response_data: OpenAIResponse = response
        .json()
        .context("Failed to parse ChatGPT response")?;

    Ok(response_data.choices[0].message.content.clone())
}

fn send_claude(settings: &Settings, prompt: &str) -> Result<String> {
    let config = &settings.providers.claude;

    if config.api_key.is_empty() {
        anyhow::bail!("Claude api_key is empty. Please configure in ~/.onyx/settings.toml");
    }

    let request = ClaudeRequest {
        model: config.model.clone(),
        max_tokens: config.max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        temperature: config.temperature,
        top_p: config.top_p,
        top_k: config.top_k,
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .context("Failed to send request to Claude")?;

    let response_data: ClaudeResponse = response
        .json()
        .context("Failed to parse Claude response")?;

    Ok(response_data.content[0].text.clone())
}

fn send_ollama(settings: &Settings, prompt: &str) -> Result<String> {
    let config = &settings.providers.ollama;

    let request = OllamaRequest {
        model: config.model.clone(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: config.temperature,
            num_predict: config.num_predict,
            top_k: config.top_k,
            top_p: config.top_p,
            repeat_penalty: config.repeat_penalty,
        },
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .context("Failed to send request to Ollama")?;

    let response_data: OllamaResponse = response
        .json()
        .context("Failed to parse Ollama response")?;

    Ok(response_data.response)
}

fn send_llamacpp(settings: &Settings, prompt: &str) -> Result<String> {
    let config = &settings.providers.llamacpp;

    let request = LlamaCppRequest {
        prompt: prompt.to_string(),
        temperature: config.temperature,
        n_predict: config.n_predict,
        top_k: config.top_k,
        top_p: config.top_p,
        repeat_penalty: config.repeat_penalty,
        repeat_last_n: config.repeat_last_n,
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(&config.endpoint)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .context("Failed to send request to llama.cpp")?;

    let response_data: serde_json::Value = response
        .json()
        .context("Failed to parse llama.cpp response")?;

    let content = response_data["content"]
        .as_str()
        .context("Missing content field in llama.cpp response")?;

    Ok(content.to_string())
}
