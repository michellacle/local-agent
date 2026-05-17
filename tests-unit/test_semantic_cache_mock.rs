use local_agent::capability_router::{ActionType, ResourceType, RoutingIntent};
use local_agent::semantic_cache::{
    CachedEntry, InMemorySemanticCache, MockSemanticCache, SemanticCache,
};
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
fn test_mock_lookup_exact_match() {
    let mut cache = MockSemanticCache::new(0.92);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store(
        "query",
        &emb,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    let result = cache.lookup(&emb);
    assert!(result.is_some());
    assert_eq!(result.unwrap().action, ActionType::Find);
}

#[test]
fn test_mock_lookup_no_match() {
    let mut cache = MockSemanticCache::new(0.92);
    let emb_a = embedding(&[1.0, 0.0, 0.0]);
    let emb_b = embedding(&[0.0, 1.0, 0.0]);
    cache.store(
        "query",
        &emb_a,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    let result = cache.lookup(&emb_b);
    assert!(result.is_none());
}

#[test]
fn test_mock_with_preloaded_entries() {
    let entries = vec![CachedEntry::new(
        "preloaded".into(),
        embedding(&[1.0, 0.0, 0.0]),
        intent(ActionType::Analyze, ResourceType::SalesReport),
    )];
    let mut cache = MockSemanticCache::with_entries(0.92, entries);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    let result = cache.lookup(&emb);
    assert!(result.is_some());
    assert_eq!(result.unwrap().action, ActionType::Analyze);
}

#[test]
fn test_mock_no_eviction() {
    let mut cache = MockSemanticCache::new(0.92);
    for i in 0..10 {
        cache.store(
            &format!("q{i}"),
            &embedding(&[i as f64, 0.0, 0.0]),
            &intent(ActionType::Find, ResourceType::SalesReport),
        );
    }
    assert_eq!(cache.size(), 10);
}

#[test]
fn test_mock_clear() {
    let mut cache = MockSemanticCache::new(0.92);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.clear();
    assert_eq!(cache.size(), 0);
}

#[test]
fn test_mock_threshold() {
    let cache = MockSemanticCache::new(0.85);
    assert_eq!(cache.threshold(), 0.85);
}

#[test]
fn test_mock_entries_returns_slice() {
    let mut cache = MockSemanticCache::new(0.92);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store(
        "q",
        &emb,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    assert_eq!(cache.entries().len(), 1);
    assert_eq!(cache.entries()[0].query, "q");
}

#[test]
fn test_mock_hit_count_increments() {
    let mut cache = MockSemanticCache::new(0.92);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store(
        "q",
        &emb,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.lookup(&emb);
    cache.lookup(&emb);
    cache.lookup(&emb);
    assert_eq!(cache.entries()[0].hit_count, 3);
}

#[test]
fn test_in_memory_close_noop() {
    let mut cache = InMemorySemanticCache::new(0.92, 100);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.close();
    assert_eq!(cache.size(), 1);
}

#[test]
fn test_mock_close_noop() {
    let mut cache = MockSemanticCache::new(0.92);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.close();
    assert_eq!(cache.size(), 1);
}
