//! Phase 3: Inverted index (word -> set of URLs). Phase 4: save/load. Phase 6: TF-IDF ranking.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::crawler::CrawlResult;
use crate::tokenize;

/// Inverted index: word -> URLs containing that word (backward compat / simple search).
pub type InvertedIndex = HashMap<String, HashSet<String>>;

/// Index with term frequency per document for TF-IDF ranking.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct IndexWithTf {
    /// term -> url -> term count in that document
    pub term_tf: HashMap<String, HashMap<String, u32>>,
    /// Total number of documents
    pub doc_count: usize,
}

impl IndexWithTf {
    /// Build from crawl results.
    pub fn build(results: &[CrawlResult]) -> Self {
        let mut term_tf: HashMap<String, HashMap<String, u32>> = HashMap::new();
        for result in results {
            let words = tokenize::tokenize(&result.body_text);
            for word in words {
                *term_tf
                    .entry(word)
                    .or_default()
                    .entry(result.url.clone())
                    .or_insert(0) += 1;
            }
        }
        let doc_count = results.len();
        Self { term_tf, doc_count }
    }

    /// As simple InvertedIndex (word -> set of URLs) for backward compat.
    pub fn as_inverted(&self) -> InvertedIndex {
        self.term_tf
            .iter()
            .map(|(k, v)| (k.clone(), v.keys().cloned().collect()))
            .collect()
    }

    /// Search with TF-IDF ranking. Returns (url, score) sorted by score descending.
    pub fn search_ranked(&self, query: &str) -> Vec<(String, f64)> {
        let words = tokenize::tokenize(query);
        if words.is_empty() || self.doc_count == 0 {
            return Vec::new();
        }
        let n = self.doc_count as f64;
        let mut url_scores: HashMap<String, f64> = HashMap::new();
        for word in &words {
            let url_counts = match self.term_tf.get(word) {
                Some(m) => m,
                None => continue,
            };
            let df = url_counts.len() as f64;
            let idf = (n + 1.0) / (df + 1.0);
            let idf = idf.ln() + 1.0;
            for (url, &tf) in url_counts {
                *url_scores.entry(url.clone()).or_insert(0.0) += (tf as f64) * idf;
            }
        }
        let mut v: Vec<(String, f64)> = url_scores.into_iter().collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

/// Build inverted index from crawl results (simple, no TF).
pub fn build_index(results: &[CrawlResult]) -> InvertedIndex {
    IndexWithTf::build(results).as_inverted()
}

/// Build index with TF for ranking; save as IndexWithTf.
pub fn build_index_with_tf(results: &[CrawlResult]) -> IndexWithTf {
    IndexWithTf::build(results)
}

/// Look up URLs that contain the given query (simple, no ranking).
pub fn search(index: &InvertedIndex, query: &str) -> Vec<String> {
    let words = tokenize::tokenize(query);
    if words.is_empty() {
        return Vec::new();
    }
    let mut urls: Option<HashSet<String>> = None;
    for word in &words {
        let set = index.get(word).cloned().unwrap_or_default();
        urls = Some(match urls {
            None => set,
            Some(u) => u.intersection(&set).cloned().collect(),
        });
    }
    urls.map(|u| {
        let mut v: Vec<String> = u.into_iter().collect();
        v.sort();
        v
    }).unwrap_or_default()
}

/// Save index (simple InvertedIndex) to JSON file.
pub fn save_index(index: &InvertedIndex, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(index)?;
    fs::write(path, json)?;
    Ok(())
}

/// Save IndexWithTf to JSON file.
pub fn save_index_with_tf(index: &IndexWithTf, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let json = serde_json::to_string_pretty(index)?;
    fs::write(path, json)?;
    Ok(())
}

/// Load index from JSON file. Tries IndexWithTf first, then falls back to InvertedIndex.
pub fn load_index(path: &Path) -> Result<InvertedIndex, Box<dyn std::error::Error + Send + Sync>> {
    let json = fs::read_to_string(path)?;
    if let Ok(with_tf) = serde_json::from_str::<IndexWithTf>(&json) {
        return Ok(with_tf.as_inverted());
    }
    let index: InvertedIndex = serde_json::from_str(&json)?;
    Ok(index)
}

/// Load IndexWithTf from JSON file (for ranked search).
pub fn load_index_with_tf(path: &Path) -> Result<IndexWithTf, Box<dyn std::error::Error + Send + Sync>> {
    let json = fs::read_to_string(path)?;
    let index = serde_json::from_str(&json)?;
    Ok(index)
}
