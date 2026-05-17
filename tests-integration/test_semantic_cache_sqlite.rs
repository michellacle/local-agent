use local_agent::capability_router::{ActionType, ResourceType, RoutingIntent};
use local_agent::semantic_cache::{SemanticCache, SqliteSemanticCache};
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

fn temp_db() -> tempfile::NamedTempFile {
    tempfile::Builder::new()
        .prefix("cache")
        .suffix(".db")
        .tempfile()
        .unwrap()
}

#[test]
fn test_sqlite_lookup_exact_match() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 1000, &path);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store(
        "query",
        &emb,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    let result = cache.lookup(&emb);
    assert!(result.is_some());
    assert_eq!(result.unwrap().action, ActionType::Find);
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_lookup_no_match() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 1000, &path);
    let emb_a = embedding(&[1.0, 0.0, 0.0]);
    let emb_b = embedding(&[0.0, 1.0, 0.0]);
    cache.store(
        "query",
        &emb_a,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    let result = cache.lookup(&emb_b);
    assert!(result.is_none());
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_hit_count_increments() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 1000, &path);
    let emb = embedding(&[1.0, 0.0, 0.0]);
    cache.store(
        "query",
        &emb,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.lookup(&emb);
    cache.lookup(&emb);
    assert_eq!(cache.entries()[0].hit_count, 2);
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_clear() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 1000, &path);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.clear();
    assert_eq!(cache.size(), 0);
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_threshold() {
    let path = temp_db();
    let cache = SqliteSemanticCache::new(0.85, 100, &path);
    assert_eq!(cache.threshold(), 0.85);
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_eviction_keeps_most_hit() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 2, &path);
    let emb_a = embedding(&[1.0, 0.0, 0.0]);
    let emb_b = embedding(&[0.0, 1.0, 0.0]);
    let emb_c = embedding(&[0.0, 0.0, 1.0]);
    cache.store(
        "a",
        &emb_a,
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.store(
        "b",
        &emb_b,
        &intent(ActionType::Analyze, ResourceType::SalesReport),
    );
    cache.lookup(&emb_b);
    cache.lookup(&emb_b);
    cache.store(
        "c",
        &emb_c,
        &intent(ActionType::Create, ResourceType::SalesReport),
    );
    assert_eq!(cache.size(), 2);
    let remaining: Vec<&str> = cache.entries().iter().map(|e| e.query.as_str()).collect();
    assert!(remaining.contains(&"b"));
    assert!(!remaining.contains(&"a"));
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_persistence_reload() {
    let path = temp_db();
    let emb = embedding(&[0.1, 0.2, 0.3, 0.4, 0.5]);
    let intent = intent(ActionType::Find, ResourceType::SalesReport);

    {
        let mut cache1 = SqliteSemanticCache::new(0.92, 1000, &path);
        cache1.store("Find the Q1 sales report", &emb, &intent);
        assert_eq!(cache1.size(), 1);
        cache1.close();
    }

    {
        let mut cache2 = SqliteSemanticCache::new(0.92, 1000, &path);
        let cached = cache2.lookup(&emb);
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.action, ActionType::Find);
        assert_eq!(cached.resource, ResourceType::SalesReport);
        assert_eq!(cache2.size(), 1);
        cache2.close();
    }
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_entries_returns_slice() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 100, &path);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    assert_eq!(cache.entries().len(), 1);
    assert_eq!(cache.entries()[0].query, "q");
    cache.close();
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_sqlite_close_releases_conn() {
    let path = temp_db();
    let mut cache = SqliteSemanticCache::new(0.92, 1000, &path);
    cache.store(
        "q",
        &embedding(&[1.0]),
        &intent(ActionType::Find, ResourceType::SalesReport),
    );
    cache.close();
    // After close, the connection is dropped; the file should still exist
    assert!(std::path::Path::new(&path).exists());
    std::fs::remove_file(&path).ok();
}
