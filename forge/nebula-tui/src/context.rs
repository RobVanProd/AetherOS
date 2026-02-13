use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Tracks user session context for smarter proactive intelligence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionContext {
    /// Keywords extracted from user queries, with frequency counts.
    pub topics: HashMap<String, u32>,
    /// Categories the user has dismissed (reduced proactive frequency).
    pub dismissed_categories: Vec<String>,
    /// Total queries this session.
    pub query_count: u32,
    /// Last few queries (up to 10).
    pub recent_queries: Vec<String>,
    /// Session start time (not serialized — set on load).
    #[serde(skip)]
    pub session_start: Option<Instant>,
    /// Last save time.
    #[serde(skip)]
    last_save: Option<Instant>,
}

const SESSION_FILE: &str = "/tmp/aether_session.json";
const SAVE_INTERVAL_SECS: u64 = 60;

/// Stop words that don't count as topics.
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "must", "ought",
    "i", "me", "my", "mine", "we", "our", "ours", "you", "your", "yours",
    "he", "him", "his", "she", "her", "hers", "it", "its", "they", "them",
    "their", "theirs", "what", "which", "who", "whom", "this", "that",
    "these", "those", "am", "at", "by", "for", "with", "about", "against",
    "between", "through", "during", "before", "after", "above", "below",
    "to", "from", "up", "down", "in", "out", "on", "off", "over", "under",
    "again", "further", "then", "once", "here", "there", "when", "where",
    "why", "how", "all", "both", "each", "few", "more", "most", "other",
    "some", "such", "no", "nor", "not", "only", "own", "same", "so",
    "than", "too", "very", "just", "don", "now", "of", "and", "but", "or",
    "if", "tell", "show", "get", "give", "make", "go", "know", "take",
    "see", "come", "think", "look", "want", "say", "use", "find", "put",
];

impl SessionContext {
    pub fn new() -> Self {
        Self {
            topics: HashMap::new(),
            dismissed_categories: Vec::new(),
            query_count: 0,
            recent_queries: Vec::new(),
            session_start: Some(Instant::now()),
            last_save: None,
        }
    }

    /// Load from disk, or create new if file doesn't exist / is corrupt.
    pub fn load() -> Self {
        match std::fs::read_to_string(SESSION_FILE) {
            Ok(data) => {
                let mut ctx: Self = serde_json::from_str(&data).unwrap_or_else(|_| Self::new());
                ctx.session_start = Some(Instant::now());
                ctx
            }
            Err(_) => Self::new(),
        }
    }

    /// Save to disk if enough time has passed since last save.
    pub fn maybe_save(&mut self) {
        let should_save = match self.last_save {
            Some(last) => last.elapsed().as_secs() >= SAVE_INTERVAL_SECS,
            None => true,
        };
        if should_save {
            self.save();
        }
    }

    /// Force save to disk.
    fn save(&mut self) {
        if let Ok(data) = serde_json::to_string(self) {
            let _ = std::fs::write(SESSION_FILE, data);
        }
        self.last_save = Some(Instant::now());
    }

    /// Record a user query — extracts keywords and updates topics.
    pub fn record_query(&mut self, query: &str) {
        self.query_count += 1;
        self.recent_queries.push(query.to_string());
        if self.recent_queries.len() > 10 {
            self.recent_queries.remove(0);
        }

        // Extract keywords
        let words: Vec<&str> = query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .collect();

        for word in words {
            let lower = word.to_lowercase();
            if !STOP_WORDS.contains(&lower.as_str()) {
                *self.topics.entry(lower).or_insert(0) += 1;
            }
        }

        // Cap topics at 50 most frequent
        if self.topics.len() > 50 {
            let mut entries: Vec<(String, u32)> = self.topics.drain().collect();
            entries.sort_by(|a, b| b.1.cmp(&a.1));
            entries.truncate(50);
            self.topics = entries.into_iter().collect();
        }
    }

    /// Record a dismissed category to reduce proactive noise.
    pub fn record_dismiss(&mut self, category: &str) {
        if !self.dismissed_categories.contains(&category.to_string()) {
            self.dismissed_categories.push(category.to_string());
        }
    }

    /// Get top N topics by frequency.
    pub fn top_topics(&self, n: usize) -> Vec<String> {
        let mut entries: Vec<(&String, &u32)> = self.topics.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.into_iter().take(n).map(|(k, _)| k.clone()).collect()
    }
}
