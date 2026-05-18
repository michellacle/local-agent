use std::collections::HashMap;
use std::time::Duration;

/// Trait for sending chat and structured-output requests to an LLM.
pub trait LLMClientTrait: Send {
    /// Send a chat request and return the raw text response.
    fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        system_prompt: Option<&str>,
        response_format: Option<serde_json::Value>,
    ) -> Result<String, String>;

    /// Send a structured chat request and return the parsed JSON response.
    fn structured_chat(
        &self,
        messages: Vec<serde_json::Value>,
        schema: serde_json::Value,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String>;
}

impl LLMClientTrait for LLMClient {
    fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        system_prompt: Option<&str>,
        response_format: Option<serde_json::Value>,
    ) -> Result<String, String> {
        LLMClient::chat(self, messages, system_prompt, response_format)
    }

    fn structured_chat(
        &self,
        messages: Vec<serde_json::Value>,
        schema: serde_json::Value,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        LLMClient::structured_chat(self, messages, schema, system_prompt)
    }
}

/// Trait for generating text embeddings.
pub trait EmbeddingClientTrait: Send {
    /// Generate an embedding vector for the given text.
    fn embed(&self, text: &str) -> Result<Vec<f64>, String>;
}

impl EmbeddingClientTrait for EmbeddingClient {
    fn embed(&self, text: &str) -> Result<Vec<f64>, String> {
        EmbeddingClient::embed(self, text)
    }
}

/// Configuration for connecting to an LLM provider.
#[derive(Debug, Clone)]
pub struct LLMConfig {
    /// Identifier for the provider (e.g., "ollama-local", "openai_compat").
    pub provider: String,
    /// Base URL of the provider's API endpoint.
    pub base_url: String,
    /// Model name to use for completions.
    pub model: String,
    /// API key for authentication.
    pub api_key: String,
    /// Sampling temperature for response generation.
    pub temperature: f64,
    /// Maximum number of tokens in the response.
    pub max_tokens: u32,
    /// Request timeout in seconds.
    pub timeout: u64,
    /// When true, forces deterministic output (temperature=0, seed=0).
    pub deterministic: bool,
}

impl LLMConfig {
    pub fn ollama(host: &str, model: &str, deterministic: bool) -> Self {
        Self {
            provider: "ollama-local".into(),
            base_url: format!("http://{host}:11434/v1"),
            model: model.to_string(),
            api_key: "not-needed".into(),
            temperature: 0.0,
            max_tokens: 2048,
            timeout: 30,
            deterministic,
        }
    }

    pub fn prepare(&self) -> HashMap<String, serde_json::Value> {
        let mut payload = HashMap::new();
        payload.insert(
            "model".into(),
            serde_json::Value::String(self.model.clone()),
        );
        payload.insert(
            "temperature".into(),
            if self.deterministic {
                serde_json::Value::Number(serde_json::Number::from_f64(0.0).unwrap())
            } else {
                serde_json::Value::Number(serde_json::Number::from_f64(self.temperature).unwrap())
            },
        );
        payload.insert(
            "max_tokens".into(),
            serde_json::Value::Number(serde_json::Number::from(self.max_tokens)),
        );
        if self.deterministic {
            payload.insert(
                "seed".into(),
                serde_json::Value::Number(serde_json::Number::from(0)),
            );
        }
        payload
    }
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: "openai_compat".into(),
            base_url: std::env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434/v1".into()),
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen3.5:2b".into()),
            api_key: std::env::var("LLM_API_KEY").unwrap_or_else(|_| "not-needed".into()),
            temperature: 0.0,
            max_tokens: 2048,
            timeout: 30,
            deterministic: false,
        }
    }
}

/// Configuration for connecting to an embedding provider.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Identifier for the embedding provider.
    pub provider: String,
    /// Base URL of the embedding API endpoint.
    pub base_url: String,
    /// Embedding model name.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout: u64,
}

impl EmbeddingConfig {
    pub fn ollama(host: &str, model: &str) -> Self {
        Self {
            provider: "ollama-embedding".into(),
            base_url: format!("http://{host}:11434"),
            model: model.to_string(),
            timeout: 30,
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "ollama-embedding".into(),
            base_url: std::env::var("EMBEDDING_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".into()),
            model: std::env::var("EMBEDDING_MODEL").unwrap_or_else(|_| "nomic-embed-text".into()),
            timeout: 30,
        }
    }
}

/// HTTP client for sending chat and structured-output requests to an LLM.
pub struct LLMClient {
    /// Connection and model configuration.
    config: LLMConfig,
    /// Underlying HTTP agent.
    agent: ureq::Agent,
}

impl LLMClient {
    pub fn new(config: Option<LLMConfig>) -> Self {
        let config = config.unwrap_or_default();
        let agent = ureq::Agent::new();
        Self { agent, config }
    }

    pub fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        system_prompt: Option<&str>,
        response_format: Option<serde_json::Value>,
    ) -> Result<String, String> {
        let mut msg_list = messages;
        if let Some(prompt) = system_prompt {
            let sys = serde_json::json!({"role": "system", "content": prompt});
            msg_list.insert(0, sys);
        }

        let mut payload = self.config.prepare();
        payload.insert("messages".into(), serde_json::Value::Array(msg_list));

        if let Some(fmt) = response_format {
            payload.insert("response_format".into(), fmt);
        }

        let url = format!("{}/chat/completions", self.config.base_url);
        let mut req = self.agent.post(&url);
        req = req.set("Content-Type", "application/json");
        req = req.set("Authorization", &format!("Bearer {}", self.config.api_key));

        let resp = req
            .timeout(Duration::from_secs(self.config.timeout))
            .send_json(serde_json::Value::Object(payload.into_iter().collect()))
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let data: serde_json::Value = resp
            .into_json::<serde_json::Value>()
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid response format from LLM".into())
    }

    pub fn structured_chat(
        &self,
        messages: Vec<serde_json::Value>,
        schema: serde_json::Value,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if self.config.provider == "ollama-local" {
            return self.ollama_structured_chat(messages, schema, system_prompt);
        }

        let response_format = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "structured_output",
                "schema": schema,
                "strict": true
            }
        });

        let raw = self.chat(messages, system_prompt, Some(response_format))?;
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse structured output: {e}"))
    }

    fn ollama_structured_chat(
        &self,
        messages: Vec<serde_json::Value>,
        schema: serde_json::Value,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut msg_list = messages;
        let schema_prompt = format!(
            "{}\n\nReturn exactly one JSON object and no other text. The JSON must conform to this schema:\n{}",
            system_prompt.unwrap_or(""),
            schema
        );
        msg_list.insert(
            0,
            serde_json::json!({"role": "system", "content": schema_prompt}),
        );

        let base_url = self
            .config
            .base_url
            .strip_suffix("/v1")
            .unwrap_or(&self.config.base_url);
        let url = format!("{base_url}/api/chat");
        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": msg_list,
            "stream": false,
            "think": false,
            "format": "json",
            "options": {
                "temperature": if self.config.deterministic { 0.0 } else { self.config.temperature },
                "num_predict": self.config.max_tokens,
                "seed": if self.config.deterministic { 0 } else { -1 }
            }
        });

        let resp = self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(self.config.timeout))
            .send_json(payload)
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let data: serde_json::Value = resp
            .into_json::<serde_json::Value>()
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        let content = data["message"]["content"]
            .as_str()
            .ok_or_else(|| "Invalid response format from Ollama".to_string())?;

        serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse structured output: {e}; content={content:?}"))
    }
}

/// HTTP client for generating text embeddings from an embedding provider.
pub struct EmbeddingClient {
    /// Embedding provider configuration.
    config: EmbeddingConfig,
    /// Underlying HTTP agent.
    agent: ureq::Agent,
}

impl EmbeddingClient {
    pub fn new(config: Option<EmbeddingConfig>) -> Self {
        let config = config.unwrap_or_default();
        let agent = ureq::Agent::new();
        Self { agent, config }
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f64>, String> {
        let url = format!("{}/api/embeddings", self.config.base_url);
        let payload = serde_json::json!({
            "model": self.config.model,
            "prompt": text
        });

        let resp = self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .timeout(Duration::from_secs(self.config.timeout))
            .send_json(payload)
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let data: serde_json::Value = resp
            .into_json::<serde_json::Value>()
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        data["embedding"]
            .as_array()
            .and_then(|arr| arr.iter().map(|v| v.as_f64()).collect::<Option<Vec<f64>>>())
            .ok_or_else(|| "Invalid embedding response".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = LLMConfig::default();
        assert_eq!(config.provider, "openai_compat");
        assert_eq!(config.temperature, 0.0);
        assert_eq!(config.max_tokens, 2048);
    }

    #[test]
    fn test_config_ollama() {
        let config = LLMConfig::ollama("localhost", "qwen3.5:2b", false);
        assert_eq!(config.base_url, "http://localhost:11434/v1");
        assert_eq!(config.model, "qwen3.5:2b");
    }

    #[test]
    fn test_config_ollama_custom_host() {
        let config = LLMConfig::ollama("minadioro", "qwen3.5:2b", false);
        assert_eq!(config.base_url, "http://minadioro:11434/v1");
    }

    #[test]
    fn test_embedding_config_ollama() {
        let config = EmbeddingConfig::ollama("localhost", "nomic-embed-text");
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "nomic-embed-text");
    }

    #[test]
    fn test_llm_config_deterministic_prepare() {
        let config = LLMConfig::ollama("localhost", "qwen3.5:2b", true);
        let payload = config.prepare();
        assert_eq!(payload.get("temperature").unwrap().as_f64().unwrap(), 0.0);
        assert_eq!(payload.get("seed").unwrap().as_i64().unwrap(), 0);
    }

    #[test]
    fn test_llm_config_non_deterministic_prepare() {
        let config = LLMConfig::ollama("localhost", "qwen3.5:2b", false);
        let payload = config.prepare();
        assert!(!payload.contains_key("seed"));
    }
}
