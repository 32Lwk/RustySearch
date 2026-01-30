//! Text to word tokenization: split, strip punctuation, lowercase.

/// Split text into words: by whitespace, strip non-alphanumeric, lowercase.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|s| {
            s.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|s| !s.is_empty())
        .collect()
}
