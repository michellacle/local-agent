use local_agent::capability_router::{ActionType, ResourceType, RoutingIntent};
use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::{EmbeddingClientTrait, LLMClientTrait};
use local_agent::semantic_cache::{InMemorySemanticCache, SemanticCache};
use serde_json::json;

// --- Mock LLM Client ---

struct MockLLMClient {
    structured_response: Result<serde_json::Value, String>,
    chat_response: Result<String, String>,
}

impl MockLLMClient {
    fn new_valid_intent() -> Self {
        Self {
            structured_response: Ok(json!({
                "action": "find",
                "resource": "sales_report",
                "parameters": {"quarter": "Q1"}
            })),
            chat_response: Err("should not reach chat fallback".into()),
        }
    }

    fn new_error() -> Self {
        Self {
            structured_response: Err("LLM error".into()),
            chat_response: Err("chat also fails".into()),
        }
    }

    fn new_fallback_valid() -> Self {
        Self {
            structured_response: Err("structured fails, try fallback".into()),
            chat_response: Ok(json!({
                "action": "analyze",
                "resource": "server_log",
                "parameters": {"date": "2024-01-01"}
            })
            .to_string()),
        }
    }

    fn new_invalid_action() -> Self {
        Self {
            structured_response: Ok(json!({
                "action": "unknown_action",
                "resource": "sales_report",
                "parameters": {}
            })),
            chat_response: Err("should not reach chat fallback".into()),
        }
    }

    fn new_invalid_resource() -> Self {
        Self {
            structured_response: Ok(json!({
                "action": "find",
                "resource": "unknown_resource",
                "parameters": {}
            })),
            chat_response: Err("should not reach chat fallback".into()),
        }
    }
}

impl LLMClientTrait for MockLLMClient {
    fn chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _system_prompt: Option<&str>,
        _response_format: Option<serde_json::Value>,
    ) -> Result<String, String> {
        self.chat_response.clone()
    }

    fn structured_chat(
        &self,
        _messages: Vec<serde_json::Value>,
        _schema: serde_json::Value,
        _system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        self.structured_response.clone()
    }
}

// --- Mock Embedding Client ---

struct MockEmbeddingClient {
    embedding: Vec<f64>,
}

impl MockEmbeddingClient {
    fn new(embedding: Vec<f64>) -> Self {
        Self { embedding }
    }
}

impl EmbeddingClientTrait for MockEmbeddingClient {
    fn embed(&self, _text: &str) -> Result<Vec<f64>, String> {
        Ok(self.embedding.clone())
    }
}

// --- Existing tests (updated for trait-based constructor) ---

#[test]
fn test_intent_schema_has_required_fields() {
    let _intent = RoutingIntent {
        action: ActionType::Find,
        resource: ResourceType::SalesReport,
        parameters: json!({"key": "value"}),
    };

    let mut schema = schemars::schema_for!(RoutingIntent);
    let obj_schema = schema.schema.object();
    assert!(obj_schema.properties.contains_key("action"));
    assert!(obj_schema.properties.contains_key("resource"));
    assert!(obj_schema.properties.contains_key("parameters"));
    assert!(obj_schema.required.contains("action"));
    assert!(obj_schema.required.contains("resource"));
}

#[test]
fn test_system_prompt_exists() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
    let _ = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
}

// --- Extract method tests ---

#[test]
fn test_extract_valid_intent() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let intent = extractor.extract("Find the Q1 sales report").unwrap();
    assert_eq!(intent.action, ActionType::Find);
    assert_eq!(intent.resource, ResourceType::SalesReport);
    assert_eq!(intent.parameters["quarter"], "Q1");
}

#[test]
fn test_extract_llm_error_propagates() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_error());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let result = extractor.extract("Find the Q1 sales report");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("chat also fails"));
}

#[test]
fn test_extract_fallback_to_chat() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_fallback_valid());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let intent = extractor.extract("Analyze the server log").unwrap();
    assert_eq!(intent.action, ActionType::Analyze);
    assert_eq!(intent.resource, ResourceType::ServerLog);
    assert_eq!(intent.parameters["date"], "2024-01-01");
}

#[test]
fn test_extract_invalid_action() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_invalid_action());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let result = extractor.extract("Find the Q1 sales report");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid action"));
}

#[test]
fn test_extract_invalid_resource() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_invalid_resource());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let result = extractor.extract("Find the Q1 sales report");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid resource"));
}

#[test]
fn test_extract_cache_hit_returns_cached_intent() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());

    let embedding = vec![1.0, 0.0, 0.0];
    let cached_intent = RoutingIntent {
        action: ActionType::Create,
        resource: ResourceType::Document,
        parameters: json!({"topic": "audit"}),
    };

    let mut cache = InMemorySemanticCache::new(0.9, 100);
    cache.store("Create an audit document", &embedding, &cached_intent);

    let embed_client: Box<dyn EmbeddingClientTrait> =
        Box::new(MockEmbeddingClient::new(embedding.clone()));

    let mut extractor = IntentExtractor::new(client, Some(embed_client), Some(Box::new(cache)));

    let intent = extractor.extract("Create an audit document").unwrap();
    assert_eq!(intent.action, ActionType::Create);
    assert_eq!(intent.resource, ResourceType::Document);
    assert_eq!(intent.parameters["topic"], "audit");
}

#[test]
fn test_extract_cache_miss_calls_llm_and_stores() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());

    let mut cache = InMemorySemanticCache::new(0.9, 100);
    cache.store(
        "Different query",
        &[0.0, 1.0, 0.0],
        &RoutingIntent {
            action: ActionType::Find,
            resource: ResourceType::ServerLog,
            parameters: json!({}),
        },
    );

    let embedding = vec![1.0, 0.0, 0.0];
    let embed_client: Box<dyn EmbeddingClientTrait> =
        Box::new(MockEmbeddingClient::new(embedding.clone()));

    let mut extractor = IntentExtractor::new(client, Some(embed_client), Some(Box::new(cache)));

    let intent = extractor.extract("Find the Q1 sales report").unwrap();
    assert_eq!(intent.action, ActionType::Find);
    assert_eq!(intent.resource, ResourceType::SalesReport);
}

#[test]
fn test_extract_no_cache_calls_llm() {
    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClient::new_valid_intent());
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let intent = extractor.extract("Find the Q1 sales report").unwrap();
    assert_eq!(intent.action, ActionType::Find);
    assert_eq!(intent.resource, ResourceType::SalesReport);
}

#[test]
fn test_extract_missing_parameters_defaults_to_empty() {
    struct MockLLMClientNoParams;

    impl LLMClientTrait for MockLLMClientNoParams {
        fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _system_prompt: Option<&str>,
            _response_format: Option<serde_json::Value>,
        ) -> Result<String, String> {
            Err("should not reach chat fallback".into())
        }

        fn structured_chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _schema: serde_json::Value,
            _system_prompt: Option<&str>,
        ) -> Result<serde_json::Value, String> {
            Ok(json!({
                "action": "find",
                "resource": "document"
            }))
        }
    }

    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClientNoParams);
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let intent = extractor.extract("Find the document").unwrap();
    assert_eq!(intent.action, ActionType::Find);
    assert_eq!(intent.resource, ResourceType::Document);
    assert_eq!(intent.parameters, json!({}));
}

#[test]
fn test_extract_fallback_parse_error() {
    struct MockLLMClientFallbackParseError;

    impl LLMClientTrait for MockLLMClientFallbackParseError {
        fn chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _system_prompt: Option<&str>,
            _response_format: Option<serde_json::Value>,
        ) -> Result<String, String> {
            Ok("not valid json at all".into())
        }

        fn structured_chat(
            &self,
            _messages: Vec<serde_json::Value>,
            _schema: serde_json::Value,
            _system_prompt: Option<&str>,
        ) -> Result<serde_json::Value, String> {
            Err("structured fails".into())
        }
    }

    let client: Box<dyn LLMClientTrait> = Box::new(MockLLMClientFallbackParseError);
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);

    let result = extractor.extract("Find the Q1 sales report");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Fallback parse failed"));
}
