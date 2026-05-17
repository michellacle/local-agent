use local_agent::llm_interface::{EmbeddingConfig, LLMConfig};

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
