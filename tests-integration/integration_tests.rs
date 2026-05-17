use std::sync::OnceLock;

use local_agent::capability_router::{ActionType, ResourceType, RoutingIntent};
use local_agent::intent_extractor::IntentExtractor;
use local_agent::llm_interface::{EmbeddingConfig, LLMConfig};
use serde_json::json;

static OLLAMA_CHECK: OnceLock<()> = OnceLock::new();

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
                panic!("Ollama is not running or unreachable: {e}\nStart Ollama and ensure models qwen3.5:0.8b and nomic-embed-text are pulled.");
            }
        }
    });
}

fn format_speed(v: f64) -> String {
    format!("{:.1}", v)
}

fn make_extractor() -> IntentExtractor {
    let config = LLMConfig::ollama("localhost", "qwen3.5:0.8b", true);
    let client = local_agent::llm_interface::LLMClient::new(Some(config));
    IntentExtractor::new(
        client,
        None,
        None::<Box<dyn local_agent::semantic_cache::SemanticCache>>,
    )
}

fn make_cached_extractor() -> IntentExtractor {
    let config = LLMConfig::ollama("localhost", "qwen3.5:0.8b", true);
    let embed_config = EmbeddingConfig::ollama("localhost", "nomic-embed-text");
    let client = local_agent::llm_interface::LLMClient::new(Some(config));
    let embed_client = local_agent::llm_interface::EmbeddingClient::new(Some(embed_config));
    let cache: Box<dyn local_agent::semantic_cache::SemanticCache> = Box::new(
        local_agent::semantic_cache::InMemorySemanticCache::new(0.92, 1000),
    );
    IntentExtractor::new(client, Some(embed_client), Some(cache))
}

#[test]
fn test_find_sales_report() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor.process("Find the Q1 sales report").unwrap();
    assert_eq!(result, "SalesAgent");
}

#[test]
fn test_analyze_sales_report() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("I want to analyze our sales report performance")
        .unwrap();
    assert_eq!(result, "SalesAgent");
}

#[test]
fn test_find_document() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("I need to find the regulatory document")
        .unwrap();
    assert_eq!(result, "ComplianceAgent");
}

#[test]
fn test_create_server_log() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a server log entry for the deployment")
        .unwrap();
    assert_eq!(result, "DevOpsAgent");
}

#[test]
fn test_find_server_log_error() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Find the server log from last night")
        .unwrap();
    assert!(result.contains("Error"));
}

#[test]
fn test_create_sales_report_error() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a new sales report for Q1")
        .unwrap();
    assert!(result.contains("Error"));
}

#[test]
fn test_create_document_error() {
    ensure_ollama();
    let mut extractor = make_extractor();
    let result = extractor
        .process("Create a document summarizing the audit")
        .unwrap();
    assert!(result.contains("Error"));
}

#[test]
fn test_cache_hit_is_faster() {
    ensure_ollama();
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
    assert_eq!(routed1, "SalesAgent");
    assert_eq!(routed1, routed2);

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
    ensure_ollama();
    let mut extractor = make_cached_extractor();
    let query = "Analyze the quarterly sales report trends";

    // First call — cache miss, hits LLM
    let t0 = std::time::Instant::now();
    let result1 = extractor.process(query).unwrap();
    let first_duration = t0.elapsed();

    assert_eq!(result1, "SalesAgent");
    println!("First call (LLM): {:.2?}", first_duration);

    // Second call — cache hit, should be instant
    let t1 = std::time::Instant::now();
    let result2 = extractor.process(query).unwrap();
    let second_duration = t1.elapsed();

    assert_eq!(result2, "SalesAgent");
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
    ensure_ollama();
    let mut extractor = make_cached_extractor();
    let query = "Create a server log entry for the deployment";

    // Warm the cache
    let result1 = extractor.process(query).unwrap();
    assert_eq!(result1, "DevOpsAgent");

    // Repeat — should hit cache and return same result
    let result2 = extractor.process(query).unwrap();
    assert_eq!(result2, "DevOpsAgent");

    // Verify cache has grown
    assert!(extractor.cache.as_ref().unwrap().size() >= 1);
}

// NOTE: sqlite-specific tests have been moved to tests-integration/test_semantic_cache_sqlite.rs
