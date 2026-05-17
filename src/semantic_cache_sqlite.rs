use crate::capability_router::RoutingIntent;
use crate::semantic_cache::SemanticCache;

/// SQLite-backed semantic cache with persistence and bounded size.
pub struct SqliteSemanticCache {
    threshold: f64,
    max_size: usize,
    conn: std::sync::Mutex<rusqlite::Connection>,
    entries: Vec<crate::semantic_cache::CachedEntry>,
}

impl SqliteSemanticCache {
    pub fn new(threshold: f64, max_size: usize, db_path: &str) -> Self {
        let dir = std::path::Path::new(db_path)
            .parent()
            .expect("db_path must have a parent directory");
        std::fs::create_dir_all(dir).expect("Failed to create cache directory");

        let conn = rusqlite::Connection::open(db_path).expect("Failed to open SQLite database");
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .expect("Failed to set WAL mode");

        let mut this = Self {
            threshold,
            max_size,
            conn: std::sync::Mutex::new(conn),
            entries: Vec::new(),
        };
        this.init_db();
        this.refresh_index();
        this
    }

    fn init_db(&mut self) {
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

    fn refresh_index(&mut self) {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT query, embedding, intent, created_at, hit_count FROM cache_entries")
            .expect("Failed to prepare statement");
        let rows = stmt
            .query_map((), |row| {
                let query: String = row.get(0).unwrap();
                let emb_json: String = row.get(1).unwrap();
                let intent_json: String = row.get(2).unwrap();
                let created_at: f64 = row.get(3).unwrap();
                let hit_count: u64 = row.get(4).unwrap();
                let embedding: Vec<f64> = serde_json::from_str(&emb_json).unwrap();
                let intent: RoutingIntent = serde_json::from_str(&intent_json).unwrap();
                Ok(crate::semantic_cache::CachedEntry {
                    query,
                    embedding,
                    intent,
                    created_at,
                    hit_count,
                })
            })
            .expect("Query failed");
        self.entries = rows.filter_map(|r| r.ok()).collect();
    }

    fn evict_backend(&self, keep_n: usize) {
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
}

impl SemanticCache for SqliteSemanticCache {
    fn lookup(&mut self, embedding: &[f64]) -> Option<RoutingIntent> {
        if let Some(idx) =
            crate::semantic_cache::find_best_match(&self.entries, embedding, self.threshold)
        {
            self.entries[idx].hit_count += 1;
            let query = self.entries[idx].query.clone();
            let hit_count = self.entries[idx].hit_count;
            {
                let conn = self.conn.lock().unwrap();
                conn.execute(
                    "UPDATE cache_entries SET hit_count = ? WHERE query = ?",
                    (hit_count, query),
                )
                .expect("Failed to update hit count");
            }
            Some(self.entries[idx].intent.clone())
        } else {
            None
        }
    }

    fn store(&mut self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        self.entries.push(crate::semantic_cache::CachedEntry::new(
            query.to_string(),
            embedding.to_vec(),
            intent.clone(),
        ));

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let emb_json = serde_json::to_string(embedding).unwrap();
        let intent_json = serde_json::to_string(intent).unwrap();
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO cache_entries (query, embedding, intent, created_at, hit_count) VALUES (?, ?, ?, ?, 0)",
                (query, emb_json, intent_json, now),
            )
            .expect("Failed to insert cache entry");
        }

        while self.entries.len() > self.max_size {
            self.entries.sort_by_key(|e| e.hit_count);
            self.entries.remove(0);
            self.evict_backend(self.max_size);
        }
    }

    fn size(&self) -> usize {
        self.entries.len()
    }

    fn clear(&mut self) {
        self.entries.clear();
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM cache_entries", ())
            .expect("Failed to clear cache");
    }

    fn close(&self) {
        drop(self.conn.lock().unwrap());
    }

    fn threshold(&self) -> f64 {
        self.threshold
    }

    fn entries(&self) -> &[crate::semantic_cache::CachedEntry] {
        &self.entries
    }
}
