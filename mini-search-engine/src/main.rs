//! RustySearch: crawl, index, serve.

mod crawler;
mod index;
mod search;
mod tokenize;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use clap::{Parser, Subcommand};

const DEFAULT_INDEX_PATH: &str = "index.json";

#[derive(Parser)]
#[command(name = "mini-search-engine")]
#[command(about = "Crawl URLs, build inverted index, serve search API")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Crawl a URL and build index (saves to file).
    Crawl {
        /// Start URL to crawl (same domain only).
        #[arg(long, short)]
        url: String,

        /// Max pages to crawl.
        #[arg(long, short = 'n', default_value = "50")]
        max_pages: usize,

        /// Max depth (link hops).
        #[arg(long, short = 'd', default_value = "3")]
        max_depth: u32,

        /// Output index file path.
        #[arg(long, short, default_value = DEFAULT_INDEX_PATH)]
        output: String,
    },

    /// Load index and start search API.
    Serve {
        /// Index file path.
        #[arg(long, short, default_value = DEFAULT_INDEX_PATH)]
        index: String,

        /// Port to listen on.
        #[arg(long, short, default_value_t = 3000)]
        port: u16,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Crawl { url, max_pages, max_depth, output } => {
            run_crawl(&url, Some(max_pages), Some(max_depth), &output)?;
        }
        Command::Serve { index, port } => {
            run_serve(&index, port)?;
        }
    }
    Ok(())
}

fn run_crawl(
    url: &str,
    max_pages: Option<usize>,
    max_depth: Option<u32>,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let results = crawler::crawl(url, max_pages, max_depth)?;
    let idx = index::build_index_with_tf(&results);
    let path = Path::new(output_path);
    index::save_index_with_tf(&idx, path)?;
    println!("Crawled {} pages, index saved to {:?}", results.len(), path);
    Ok(())
}

fn run_serve(index_path: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = Path::new(index_path);
    let idx = index::load_index_with_tf(path).or_else(|_| {
        let simple = index::load_index(path)?;
        let doc_count = simple.values().flat_map(|urls| urls.iter()).collect::<std::collections::HashSet<_>>().len();
        let term_tf: HashMap<String, HashMap<String, u32>> = simple
            .into_iter()
            .map(|(term, urls)| (term, urls.into_iter().map(|u| (u, 1u32)).collect()))
            .collect();
        Ok::<index::IndexWithTf, Box<dyn std::error::Error + Send + Sync>>(index::IndexWithTf { term_tf, doc_count })
    })?;
    let state: search::AppState = Arc::new(idx);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let app = axum::Router::new()
            .route("/", axum::routing::get(search::index_page))
            .route("/search", axum::routing::get(search::search_handler))
            .with_state(state);

        let addr = format!("127.0.0.1:{}", port);
        println!("Listening on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })?;
    Ok(())
}
