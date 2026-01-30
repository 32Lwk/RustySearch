//! Phase 2: Recursive crawl within the same site.
//! Phase 5: Parallel crawl with async reqwest and Semaphore.

use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use url::Url;

const MAX_PAGES: usize = 50;
const MAX_DEPTH: u32 = 3;
const MAX_CONCURRENT: usize = 5;

/// Result of crawling a single page.
#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub url: String,
    pub title: String,
    pub body_text: String,
    pub links: Vec<String>,
}

/// Normalize URL: resolve relative path, remove fragment.
fn normalize_url(base: &Url, href: &str) -> Option<Url> {
    let parsed = base.join(href).ok()?;
    let mut url = parsed;
    url.set_fragment(None);
    Some(url)
}

/// Check if two URLs have the same domain (host).
fn same_domain(start: &Url, other: &Url) -> bool {
    start.host_str() == other.host_str()
}

/// Fetch a single page (async).
async fn fetch_page_async(
    client: &reqwest::Client,
    url: &str,
) -> Result<CrawlResult, Box<dyn std::error::Error + Send + Sync>> {
    let body = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&body);
    let base_url = Url::parse(url)?;

    let title = document
        .select(&Selector::parse("title").unwrap())
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    let body_text = document
        .select(&Selector::parse("body").unwrap())
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_default()
        .trim()
        .to_string();

    let link_selector = Selector::parse("a[href]").unwrap();
    let mut links = Vec::new();
    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(absolute) = normalize_url(&base_url, href) {
                if same_domain(&base_url, &absolute) {
                    links.push(absolute.to_string());
                }
            }
        }
    }

    Ok(CrawlResult {
        url: url.to_string(),
        title,
        body_text,
        links,
    })
}

/// Crawl starting from `start_url`, staying on the same domain (async, parallel).
async fn crawl_async(
    start_url: &str,
    max_pages: Option<usize>,
    max_depth: Option<u32>,
    max_concurrent: Option<usize>,
) -> Result<Vec<CrawlResult>, Box<dyn std::error::Error + Send + Sync>> {
    let max_pages = max_pages.unwrap_or(MAX_PAGES);
    let max_depth = max_depth.unwrap_or(MAX_DEPTH);
    let max_concurrent = max_concurrent.unwrap_or(MAX_CONCURRENT);

    let _start = Url::parse(start_url)?;
    let client = reqwest::Client::new();
    let sem = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    queue.push_back((start_url.to_string(), 0));

    let mut results = Vec::new();
    let mut join_set = tokio::task::JoinSet::new();

    loop {
        // Spawn up to max_concurrent tasks
        while results.len() + join_set.len() < max_pages {
            let (url, depth) = match queue.pop_front() {
                Some(p) => p,
                None => break,
            };
            if depth > max_depth || visited.contains(&url) {
                continue;
            }
            visited.insert(url.clone());

            let permit = sem.clone().acquire_owned().await?;
            let client = client.clone();
            let url2 = url.clone();
            join_set.spawn(async move {
                let _permit = permit;
                let r = fetch_page_async(&client, &url2).await;
                (r, depth)
            });
        }

        if join_set.is_empty() {
            break;
        }

        let Some(join_result) = join_set.join_next().await else {
            break;
        };
        let (res, depth) = join_result.map_err(|e| e.to_string())?;
        let result = match res {
            Ok(r) => r,
            Err(_) => continue,
        };
        results.push(result.clone());
        for link in &result.links {
            if !visited.contains(link) {
                queue.push_back((link.clone(), depth + 1));
            }
        }
    }

    Ok(results)
}

/// Crawl starting from `start_url`, staying on the same domain.
/// Returns at most `max_pages` results, with depth limited by `max_depth`.
/// Uses parallel async fetching (Phase 5).
pub fn crawl(
    start_url: &str,
    max_pages: Option<usize>,
    max_depth: Option<u32>,
) -> Result<Vec<CrawlResult>, Box<dyn std::error::Error + Send + Sync>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(crawl_async(start_url, max_pages, max_depth, None))
}
