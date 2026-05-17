use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LLMConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub timeout: u64,
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

#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
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

pub struct LLMClient {
    config: LLMConfig,
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
}

pub struct EmbeddingClient {
    config: EmbeddingConfig,
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
