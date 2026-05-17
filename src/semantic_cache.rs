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

/// Abstract semantic cache for matching query embeddings against stored intents.
pub trait SemanticCache: Send {
    /// Lookup a matching intent by embedding, returning None if below threshold.
    fn lookup(&mut self, embedding: &[f64]) -> Option<RoutingIntent>;
    /// Store a new query-intent mapping.
    fn store(&mut self, query: &str, embedding: &[f64], intent: &RoutingIntent);
    /// Returns the number of cached entries.
    fn size(&self) -> usize;
    /// Remove all cached entries.
    fn clear(&mut self);
    /// Close any held resources (e.g., database connections).
    fn close(&self);
    /// Returns the cosine similarity threshold for cache hits.
    fn threshold(&self) -> f64;
    /// Returns a slice of all cached entries.
    fn entries(&self) -> &[CachedEntry];
}

/// Finds the index of the best matching entry by cosine similarity, if above threshold.
pub(crate) fn find_best_match(
    entries: &[CachedEntry],
    embedding: &[f64],
    threshold: f64,
) -> Option<usize> {
    let mut best_score = -1.0_f64;
    let mut best_idx: Option<usize> = None;
    for (i, entry) in entries.iter().enumerate() {
        let score = cosine_similarity(embedding, &entry.embedding);
        if score > best_score {
            best_score = score;
            best_idx = Some(i);
        }
    }
    best_idx.filter(|_| best_score >= threshold)
}

/// In-memory semantic cache with no persistence and bounded size.
pub struct InMemorySemanticCache {
    threshold: f64,
    max_size: usize,
    entries: Vec<CachedEntry>,
}

impl InMemorySemanticCache {
    pub fn new(threshold: f64, max_size: usize) -> Self {
        Self {
            threshold,
            max_size,
            entries: Vec::new(),
        }
    }
}

impl SemanticCache for InMemorySemanticCache {
    fn lookup(&mut self, embedding: &[f64]) -> Option<RoutingIntent> {
        if let Some(idx) = find_best_match(&self.entries, embedding, self.threshold) {
            self.entries[idx].hit_count += 1;
            Some(self.entries[idx].intent.clone())
        } else {
            None
        }
    }

    fn store(&mut self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        self.entries.push(CachedEntry::new(
            query.to_string(),
            embedding.to_vec(),
            intent.clone(),
        ));
        while self.entries.len() > self.max_size {
            self.entries.sort_by_key(|e| e.hit_count);
            self.entries.remove(0);
        }
    }

    fn size(&self) -> usize {
        self.entries.len()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn close(&self) {}

    fn threshold(&self) -> f64 {
        self.threshold
    }

    fn entries(&self) -> &[CachedEntry] {
        &self.entries
    }
}

/// Mock semantic cache for testing — in-memory, no eviction, optionally pre-loaded.
pub struct MockSemanticCache {
    threshold: f64,
    entries: Vec<CachedEntry>,
}

impl MockSemanticCache {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            entries: Vec::new(),
        }
    }

    pub fn with_entries(threshold: f64, entries: Vec<CachedEntry>) -> Self {
        Self { threshold, entries }
    }
}

impl SemanticCache for MockSemanticCache {
    fn lookup(&mut self, embedding: &[f64]) -> Option<RoutingIntent> {
        if let Some(idx) = find_best_match(&self.entries, embedding, self.threshold) {
            self.entries[idx].hit_count += 1;
            Some(self.entries[idx].intent.clone())
        } else {
            None
        }
    }

    fn store(&mut self, query: &str, embedding: &[f64], intent: &RoutingIntent) {
        self.entries.push(CachedEntry::new(
            query.to_string(),
            embedding.to_vec(),
            intent.clone(),
        ));
    }

    fn size(&self) -> usize {
        self.entries.len()
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn close(&self) {}

    fn threshold(&self) -> f64 {
        self.threshold
    }

    fn entries(&self) -> &[CachedEntry] {
        &self.entries
    }
}
