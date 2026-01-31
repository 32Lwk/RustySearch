//! Phase 4: Search API handler (axum). Phase 6: TF-IDF ranked results.

use axum::extract::{Query, State};
use axum::Json;
use std::sync::Arc;

use crate::index::IndexWithTf;

/// Shared app state: index with TF for ranking.
pub type AppState = Arc<IndexWithTf>;

/// Query params for GET /search?q=...
#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// Search result: URL and TF-IDF score.
#[derive(serde::Serialize)]
pub struct SearchHit {
    pub url: String,
    pub score: f64,
}

/// GET /search?q=word -> JSON array of { url, score } sorted by score descending.
pub async fn search_handler(
    State(index): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Json<Vec<SearchHit>> {
    let ranked = index.search_ranked(&params.q);
    let hits = ranked
        .into_iter()
        .map(|(url, score)| SearchHit { url, score })
        .collect();
    Json(hits)
}

/// GET / -> static HTML search form (Phase 6 frontend).
pub async fn index_page() -> axum::response::Html<&'static str> {
    const HTML: &str = r#"
<!DOCTYPE html>
<html lang="ja">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>RustySearch</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 640px; margin: 2rem auto; padding: 0 1rem; }
    h1 { font-size: 1.5rem; }
    input[type="search"] { width: 100%; padding: 0.5rem; font-size: 1rem; box-sizing: border-box; }
    button { margin-top: 0.5rem; padding: 0.5rem 1rem; font-size: 1rem; cursor: pointer; }
    #results { margin-top: 1.5rem; }
    #results a { display: block; padding: 0.5rem 0; border-bottom: 1px solid #eee; color: #06c; }
    #results a:hover { text-decoration: underline; }
    .score { font-size: 0.875rem; color: #666; }
    .none { color: #666; }
  </style>
</head>
<body>
  <h1>RustySearch</h1>
  <form id="form">
    <input type="search" name="q" id="q" placeholder="検索語を入力" autofocus>
    <button type="submit">検索</button>
  </form>
  <div id="results"></div>
  <script>
    const form = document.getElementById('form');
    const q = document.getElementById('q');
    const results = document.getElementById('results');
    form.addEventListener('submit', async (e) => {
      e.preventDefault();
      const query = q.value.trim();
      if (!query) { results.innerHTML = ''; return; }
      results.innerHTML = '<p class="none">検索中...</p>';
      try {
        const r = await fetch('/search?q=' + encodeURIComponent(query));
        const hits = await r.json();
        if (hits.length === 0) {
          results.innerHTML = '<p class="none">該当なし</p>';
        } else {
          results.innerHTML = hits.map(h => 
            '<a href="' + h.url + '" target="_blank" rel="noopener">' + h.url + '</a>' +
            '<span class="score">score: ' + h.score.toFixed(4) + '</span>'
          ).join('');
        }
      } catch (err) {
        results.innerHTML = '<p class="none">エラー: ' + err + '</p>';
      }
    });
  </script>
</body>
</html>
"#;
    axum::response::Html(HTML)
}
