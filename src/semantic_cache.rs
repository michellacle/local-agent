use crate::capability_router::RoutingIntent;
use serde::{Deserialize, Serialize};

/// Computes the cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// A single cached query-to-intent mapping with its embedding and usage metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    /// The original natural language query text.
    pub query: String,
    /// The embedding vector of the query.
    pub embedding: Vec<f64>,
    /// The resolved routing intent for this query.
    pub intent: RoutingIntent,
    /// Unix timestamp when the entry was created.
    pub created_at: f64,
    /// Number of times this entry has been matched on lookup.
    pub hit_count: u64,
}

impl CachedEntry {
    pub fn new(query: String, embedding: Vec<f64>, intent: RoutingIntent) -> Self {
        Self {
            query,
            embedding,
            intent,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            hit_count: 0,
        }
    }
}

/// Persistent or in-memory backend for storing cached entries.
pub trait CacheStore: Send + Sync {
    /// Returns all stored entries.
    fn get_all(&self) -> Vec<(String, Vec<f64>, RoutingIntent, f64, u64)>;
    /// Inserts a new entry.
    fn add(&self, query: &str, embedding: &[f64], intent: &RoutingIntent);
    /// Removes all entries.
    fn clear(&self);
    /// Updates the hit count for a given query.
    fn update_hit(&self, query: &str, new_hit_count: u64);
    /// Evicts least-used entries, keeping only the top N by hit count.
    fn evict(&self, keep_n: usize);
    /// Closes any held resources (e.g., database connections).
    fn close(&self);
}

type CacheTuple = (String, Vec<f64>, RoutingIntent, f64, u64);

/// In-memory implementation of `CacheStore` using a thread-safe vector.
pub struct MemoryCacheStore {
    /// Thread-safe list of cached tuples.
    entries: std::sync::Mutex<Vec<CacheTuple>>,
}

impl MemoryCacheStore {
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Default for MemoryCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStore for MemoryCacheStore {
    fn get_all(&self) -> Vec<(String, Vec<f64>, RoutingIntent, f64, u64)> {
        self.entries.lock().unwrap().clone()
    }

    fn add(&self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        self.entries.lock().unwrap().push((
            query.to_string(),
            embedding.to_vec(),
            intent.clone(),
            now,
            0,
        ));
    }

    fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    fn update_hit(&self, query: &str, new_hit_count: u64) {
        let mut entries = self.entries.lock().unwrap();
        for entry in entries.iter_mut() {
            if entry.0 == query {
                entry.4 = new_hit_count;
                break;
            }
        }
    }

    fn evict(&self, keep_n: usize) {
        let mut entries = self.entries.lock().unwrap();
        entries.sort_by_key(|e| e.4);
        let len = entries.len();
        if len > keep_n {
            entries.drain(..len - keep_n);
        }
    }

    fn close(&self) {}
}

/// SQLite-backed implementation of `CacheStore` for persistent caching.
pub struct SqliteCacheStore {
    /// Thread-safe SQLite database connection.
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl SqliteCacheStore {
    pub fn new(db_path: &str) -> Self {
        let dir = std::path::Path::new(db_path)
            .parent()
            .expect("db_path must have a parent directory");
        std::fs::create_dir_all(dir).expect("Failed to create cache directory");

        let conn = rusqlite::Connection::open(db_path).expect("Failed to open SQLite database");
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .expect("Failed to set WAL mode");

        let store = Self {
            conn: std::sync::Mutex::new(conn),
        };
        store.init_db();
        store
    }

    fn init_db(&self) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS cache_entries (
                query       TEXT PRIMARY KEY,
                embedding   TEXT NOT NULL,
                intent      TEXT NOT NULL,
                created_at  REAL NOT NULL,
                hit_count   INTEGER NOT NULL DEFAULT 0
            )
            "#,
            (),
        )
        .expect("Failed to create cache table");
    }

    fn row_to_tuple(&self, row: &rusqlite::Row<'_>) -> (String, Vec<f64>, RoutingIntent, f64, u64) {
        let query: String = row.get(0).unwrap();
        let emb_json: String = row.get(1).unwrap();
        let intent_json: String = row.get(2).unwrap();
        let created_at: f64 = row.get(3).unwrap();
        let hit_count: u64 = row.get(4).unwrap();

        let embedding: Vec<f64> = serde_json::from_str(&emb_json).unwrap();
        let intent: RoutingIntent = serde_json::from_str(&intent_json).unwrap();
        (query, embedding, intent, created_at, hit_count)
    }
}

impl CacheStore for SqliteCacheStore {
    fn get_all(&self) -> Vec<(String, Vec<f64>, RoutingIntent, f64, u64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT query, embedding, intent, created_at, hit_count FROM cache_entries")
            .expect("Failed to prepare statement");
        let rows = stmt
            .query_map((), |row| Ok(self.row_to_tuple(row)))
            .expect("Query failed");
        rows.filter_map(|r| r.ok()).collect()
    }

    fn add(&self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let emb_json = serde_json::to_string(embedding).unwrap();
        let intent_json = serde_json::to_string(intent).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO cache_entries (query, embedding, intent, created_at, hit_count) VALUES (?, ?, ?, ?, 0)",
            (query, emb_json, intent_json, now),
        )
        .expect("Failed to insert cache entry");
    }

    fn clear(&self) {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM cache_entries", ())
            .expect("Failed to clear cache");
    }

    fn update_hit(&self, query: &str, new_hit_count: u64) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE cache_entries SET hit_count = ? WHERE query = ?",
            (new_hit_count, query),
        )
        .expect("Failed to update hit count");
    }

    fn evict(&self, keep_n: usize) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            DELETE FROM cache_entries
            WHERE query NOT IN (
                SELECT query FROM cache_entries ORDER BY hit_count DESC LIMIT ?
            )
            "#,
            (keep_n as i64,),
        )
        .expect("Failed to evict entries");
    }

    fn close(&self) {
        drop(self.conn.lock().unwrap());
    }
}

/// Semantic cache that matches incoming query embeddings against stored entries using cosine similarity.
pub struct SemanticCache {
    /// Minimum cosine similarity score required for a cache hit.
    threshold: f64,
    /// Maximum number of entries before eviction is triggered.
    max_size: usize,
    /// Persistent or in-memory storage backend.
    backend: Box<dyn CacheStore>,
    /// In-memory index of all cached entries for fast lookup.
    entries: Vec<CachedEntry>,
}

impl SemanticCache {
    pub fn new(threshold: f64, max_size: usize, store: &str, store_path: Option<&str>) -> Self {
        let backend: Box<dyn CacheStore> = match store {
            "sqlite" => {
                let path = match store_path {
                    Some(p) => p.to_string(),
                    None => {
                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                        format!("{}/.local/share/local-agent/cache.db", home)
                    }
                };
                Box::new(SqliteCacheStore::new(&path))
            }
            _ => Box::new(MemoryCacheStore::new()),
        };

        let mut cache = Self {
            threshold,
            max_size,
            backend,
            entries: Vec::new(),
        };
        cache.refresh_index();
        cache
    }

    pub fn with_memory(threshold: f64, max_size: usize) -> Self {
        Self::new(threshold, max_size, "memory", None)
    }

    pub fn with_sqlite(threshold: f64, max_size: usize, path: &str) -> Self {
        Self::new(threshold, max_size, "sqlite", Some(path))
    }

    fn refresh_index(&mut self) {
        let rows = self.backend.get_all();
        self.entries = rows
            .into_iter()
            .map(
                |(query, embedding, intent, created_at, hit_count)| CachedEntry {
                    query,
                    embedding,
                    intent,
                    created_at,
                    hit_count,
                },
            )
            .collect();
    }

    pub fn lookup(&mut self, embedding: &[f64]) -> Option<RoutingIntent> {
        let mut best_score = -1.0_f64;
        let mut best_idx: Option<usize> = None;

        for (i, entry) in self.entries.iter().enumerate() {
            let score = cosine_similarity(embedding, &entry.embedding);
            if score > best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx
            && best_score >= self.threshold
        {
            let entry = &mut self.entries[idx];
            entry.hit_count += 1;
            self.backend.update_hit(&entry.query, entry.hit_count);
            return Some(entry.intent.clone());
        }

        None
    }

    pub fn store(&mut self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        let entry = CachedEntry::new(query.to_string(), embedding.to_vec(), intent.clone());
        self.entries.push(entry);
        self.backend.add(query, embedding, intent);
        self.evict_if_needed();
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.backend.clear();
    }

    pub fn close(&self) {
        self.backend.close();
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > self.max_size {
            self.entries.sort_by_key(|e| e.hit_count);
            self.entries.remove(0);
            self.backend.evict(self.max_size);
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn entries(&self) -> &[CachedEntry] {
        &self.entries
    }
}
