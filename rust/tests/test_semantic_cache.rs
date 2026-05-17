use coordination_patterns::capability_router::{ActionType, ResourceType, RoutingIntent};
use coordination_patterns::semantic_cache::{SemanticCache, CachedEntry};
use serde_json::json;

fn intent(action: ActionType, resource: ResourceType) -> RoutingIntent {
    RoutingIntent {
        action,
        resource,
        parameters: json!({}),
    }
}

fn embedding(vals: &[f64]) -> Vec<f64> {
    vals.to_vec()
}

#[test]
fn test_lookup_exact_match() {
    let mut cache = SemanticCache::with_memory(0.92, 1000);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store("query", &emb, &intent(ActionType::Find, ResourceType::SalesReport));
    let result = cache.lookup(&emb);
    assert!(result.is_some());
    assert_eq!(result.unwrap().action, ActionType::Find);
}

#[test]
fn test_lookup_no_match() {
    let mut cache = SemanticCache::with_memory(0.92, 1000);
    let emb_a = embedding(&[1.0, 0.0, 0.0]);
    let emb_b = embedding(&[0.0, 1.0, 0.0]);
    cache.store("query", &emb_a, &intent(ActionType::Find, ResourceType::SalesReport));
    let result = cache.lookup(&emb_b);
    assert!(result.is_none());
}

#[test]
fn test_lookup_similar_below_threshold() {
    // Vectors with ~0.99 similarity fail a 0.995 threshold
    let mut cache = SemanticCache::with_memory(0.995, 1000);
    let emb_a = embedding(&[1.0, 0.1, 0.0]);
    let emb_b = embedding(&[1.0, 0.0, 0.1]);
    cache.store("query", &emb_a, &intent(ActionType::Find, ResourceType::SalesReport));
    let result = cache.lookup(&emb_b);
    assert!(result.is_none());
}

#[test]
fn test_lookup_hit_increments_hit_count() {
    let mut cache = SemanticCache::with_memory(0.92, 1000);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store("query", &emb, &intent(ActionType::Find, ResourceType::SalesReport));
    cache.lookup(&emb);
    cache.lookup(&emb);
    assert_eq!(cache.entries()[0].hit_count, 2);
}

#[test]
fn test_eviction_keeps_most_hit() {
    let mut cache = SemanticCache::with_memory(0.92, 2);
    let emb_a = embedding(&[1.0, 0.0, 0.0]);
    let emb_b = embedding(&[0.0, 1.0, 0.0]);
    let emb_c = embedding(&[0.0, 0.0, 1.0]);
    cache.store("a", &emb_a, &intent(ActionType::Find, ResourceType::SalesReport));
    cache.store("b", &emb_b, &intent(ActionType::Analyze, ResourceType::SalesReport));
    // Hit b twice
    cache.lookup(&emb_b);
    cache.lookup(&emb_b);
    // Adding c should evict a (hit_count=0) but keep b (hit_count=2)
    cache.store("c", &emb_c, &intent(ActionType::Create, ResourceType::SalesReport));
    assert_eq!(cache.size(), 2);
    let remaining: Vec<&str> = cache.entries().iter().map(|e| e.query.as_str()).collect();
    assert!(remaining.contains(&"b"));
    assert!(!remaining.contains(&"a"));
}

#[test]
fn test_clear() {
    let mut cache = SemanticCache::with_memory(0.92, 1000);
    cache.store("q", &embedding(&[1.0]), &intent(ActionType::Find, ResourceType::SalesReport));
    cache.clear();
    assert_eq!(cache.size(), 0);
}

#[test]
fn test_cached_entry_serialization() {
    let entry = CachedEntry::new(
        "test".into(),
        vec![1.0, 2.0],
        intent(ActionType::Find, ResourceType::SalesReport),
    );
    let data = serde_json::to_value(&entry).unwrap();
    assert_eq!(data["query"], "test");
    assert_eq!(data["embedding"], serde_json::json!([1.0, 2.0]));
}
