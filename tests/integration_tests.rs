use std::sync::{Mutex, OnceLock};

use local_agent::capability_router::{ActionType, ResourceType};
use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::{
    EmbeddingClient, EmbeddingClientTrait, EmbeddingConfig, LLMClient, LLMClientTrait, LLMConfig,
};
use local_agent::semantic_cache::{InMemorySemanticCache, SemanticCache};

static OLLAMA_CHECK: OnceLock<()> = OnceLock::new();
static SHARED_LLM_CLIENT: OnceLock<LLMClient> = OnceLock::new();
static SHARED_EMBED_CLIENT: OnceLock<EmbeddingClient> = OnceLock::new();
static SETUP: OnceLock<()> = OnceLock::new();
static LIVE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_live_test() -> std::sync::MutexGuard<'static, ()> {
    LIVE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn ensure_ollama() {
    OLLAMA_CHECK.get_or_init(|| {
        let resp = ureq::get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(5))
            .call();
        match resp {
            Ok(r) => {
                let _ = r.into_string();
            }
            Err(e) => {
                panic!("Error llm backend is not running or unreachable: {e}");
            }
        }
    });
}

fn shared_llm_config() -> LLMConfig {
    let mut config = LLMConfig::ollama("localhost", "qwen3.5:2b", true);
    config.max_tokens = 256;
    config.timeout = 120;
    config
}

fn shared_llm_client() -> &'static LLMClient {
    SHARED_LLM_CLIENT.get_or_init(|| LLMClient::new(Some(shared_llm_config())))
}

fn shared_embed_client() -> &'static EmbeddingClient {
    SHARED_EMBED_CLIENT.get_or_init(|| {
        EmbeddingClient::new(Some(EmbeddingConfig::ollama(
            "localhost",
            "nomic-embed-text",
        )))
    })
}

/// Runs once per test suite: ensures Ollama is reachable and warms up the model.
fn setup() {
    SETUP.get_or_init(|| {
        ensure_ollama();
        let client = shared_llm_client();
        let _ = client.chat(
            vec![serde_json::json!({"role": "user", "content": "ok"})],
            None,
            None,
        );
    });
}

/// Wrapper that delegates to the shared LLM client so HTTP connections are reused.
struct SharedLLMClientRef;

impl LLMClientTrait for SharedLLMClientRef {
    fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        system_prompt: Option<&str>,
        response_format: Option<serde_json::Value>,
    ) -> Result<String, String> {
        shared_llm_client().chat(messages, system_prompt, response_format)
    }

    fn structured_chat(
        &self,
        messages: Vec<serde_json::Value>,
        schema: serde_json::Value,
        system_prompt: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        shared_llm_client().structured_chat(messages, schema, system_prompt)
    }
}

/// Wrapper that delegates to the shared embedding client.
struct SharedEmbedClientRef;

impl EmbeddingClientTrait for SharedEmbedClientRef {
    fn embed(&self, text: &str) -> Result<Vec<f64>, String> {
        shared_embed_client().embed(text)
    }
}

fn format_speed(v: f64) -> String {
    format!("{:.1}", v)
}

fn make_extractor() -> IntentExtractor {
    setup();
    let client: Box<dyn LLMClientTrait> = Box::new(SharedLLMClientRef);
    IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>)
}

fn make_cached_extractor() -> IntentExtractor {
    setup();
    let client: Box<dyn LLMClientTrait> = Box::new(SharedLLMClientRef);
    let embed_client: Box<dyn EmbeddingClientTrait> = Box::new(SharedEmbedClientRef);
    let cache: Box<dyn SemanticCache> = Box::new(InMemorySemanticCache::new(0.92, 1000));
    IntentExtractor::new(client, Some(embed_client), Some(cache))
}

#[test]
fn test_find_sales_report() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor.process("Find the Q1 sales report").unwrap();
    assert_eq!(result.agent_name(), Some("SalesAgent"));
}

#[test]
fn test_analyze_sales_report() {
    let _guard = lock_live_test();
    setup();
    let client: Box<dyn LLMClientTrait> = Box::new(SharedLLMClientRef);
    let mut extractor = IntentExtractor::new(client, None, None::<Box<dyn SemanticCache>>);
    let result = extractor
        .process("I want to analyze our sales report performance")
        .unwrap();
    assert_eq!(result.agent_name(), Some("SalesAgent"));
}

#[test]
fn test_find_document() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor
        .process("I need to find the regulatory document")
        .unwrap();
    assert_eq!(result.agent_name(), Some("ComplianceAgent"));
}

#[test]
fn test_create_server_log() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a server log entry for the deployment")
        .unwrap();
    assert_eq!(result.agent_name(), Some("DevOpsAgent"));
}

#[test]
fn test_find_server_log_error() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Find the server log from last night")
        .unwrap();
    assert!(!result.is_routed());
}

#[test]
fn test_create_sales_report_error() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a new sales report for Q1")
        .unwrap();
    assert!(!result.is_routed());
}

#[test]
fn test_create_document_error() {
    let _guard = lock_live_test();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a document summarizing the audit")
        .unwrap();
    assert!(!result.is_routed());
}

#[test]
fn test_cache_hit_is_faster() {
    let _guard = lock_live_test();
    let mut extractor = make_cached_extractor();
    let query = "Find the Q1 sales report";

    // First call: cache miss
    let t0 = std::time::Instant::now();
    let intent1 = extractor.extract(query).unwrap();
    let first_elapsed = t0.elapsed();

    // Second call: should be cache hit
    let t1 = std::time::Instant::now();
    let intent2 = extractor.extract(query).unwrap();
    let second_elapsed = t1.elapsed();

    // Cache size should be 1
    assert_eq!(extractor.cache.as_ref().unwrap().size(), 1);

    // Second intent should match the first
    assert_eq!(intent2.action, intent1.action);
    assert_eq!(intent2.resource, intent1.resource);
    assert_eq!(intent2.parameters, intent1.parameters);

    // Verify the extraction is correct
    assert_eq!(intent1.action, ActionType::Find);
    assert_eq!(intent1.resource, ResourceType::SalesReport);
    assert!(intent1.parameters.get("quarter").is_some());
    assert_eq!(
        intent1.parameters.get("quarter").unwrap().as_str().unwrap(),
        "Q1"
    );

    // Verify routing
    let routed1 = extractor.process(query).unwrap();
    let routed2 = extractor.process(query).unwrap();
    assert_eq!(routed1.agent_name(), Some("SalesAgent"));
    assert_eq!(routed1.agent_name(), routed2.agent_name());

    // Second call should be significantly faster
    assert!(
        second_elapsed < first_elapsed / 3,
        "Cache hit was not fast enough: first={:?}, second={:?}",
        first_elapsed,
        second_elapsed
    );

    println!("\nFirst call  (cache miss): {:?}", first_elapsed);
    println!("Second call (cache hit) : {:?}", second_elapsed);
    let speedup = first_elapsed.as_secs_f64() / second_elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    println!("Speedup               : {}x", format_speed(speedup));
    println!(
        "Cache entries         : {}",
        extractor.cache.as_ref().unwrap().size()
    );
    println!(
        "Result                : action={}, resource={}, params={}",
        serde_json::to_value(&intent2.action).unwrap(),
        serde_json::to_value(&intent2.resource).unwrap(),
        intent2.parameters
    );
}

#[test]
fn test_cache_produces_speed_benefit() {
    let _guard = lock_live_test();
    let mut extractor = make_cached_extractor();
    let query = "Analyze the quarterly sales report trends";

    // First call — cache miss, hits LLM
    let t0 = std::time::Instant::now();
    let result1 = extractor.process(query).unwrap();
    let first_duration = t0.elapsed();

    assert_eq!(result1.agent_name(), Some("SalesAgent"));
    println!("First call (LLM): {:.2?}", first_duration);

    // Second call — cache hit, should be instant
    let t1 = std::time::Instant::now();
    let result2 = extractor.process(query).unwrap();
    let second_duration = t1.elapsed();

    assert_eq!(result2.agent_name(), Some("SalesAgent"));
    println!("Second call (cache): {:.4?}", second_duration);

    let speedup =
        first_duration.as_secs_f64() / second_duration.as_secs_f64().max(f64::MIN_POSITIVE);
    println!("Speedup: {}x", format_speed(speedup));

    assert!(
        second_duration < first_duration / 10,
        "Cache hit ({:?}) was not faster than LLM call ({:?})",
        second_duration,
        first_duration
    );
}

#[test]
fn test_cache_bypasses_llm_for_identical_query() {
    let _guard = lock_live_test();
    let mut extractor = make_cached_extractor();
    let query = "Create a server log entry for the deployment";

    // Warm the cache
    let result1 = extractor.process(query).unwrap();
    assert_eq!(result1.agent_name(), Some("DevOpsAgent"));

    // Repeat — should hit cache and return same result
    let result2 = extractor.process(query).unwrap();
    assert_eq!(result2.agent_name(), Some("DevOpsAgent"));

    // Verify cache has grown
    assert!(extractor.cache.as_ref().unwrap().size() >= 1);
}
