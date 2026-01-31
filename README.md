# RustySearch

Rust で作るミニ検索エンジン。クロール → 転置インデックス構築 → 検索 API（TF-IDF ランキング付き）を提供します。

## 必要な環境

- Rust (edition 2021, `cargo` が使えること)

## ビルド

```bash
cargo build --release
```

## 使い方

### 1. クロールしてインデックスを作成

```bash
cargo run -- crawl --url https://example.com
```

オプション:

- `--url`, `-u`: クロール開始 URL（必須）
- `--max-pages`, `-n`: 最大ページ数（既定: 50）
- `--max-depth`, `-d`: 最大リンク深さ（既定: 3）
- `--output`, `-o`: インデックス出力ファイル（既定: `index.json`）

例:

```bash
cargo run -- crawl --url https://www.rust-lang.org --max-pages 20 --output rust.json
```

### 2. 検索 API を起動

```bash
cargo run -- serve
```

オプション:

- `--index`, `-i`: 読み込むインデックスファイル（既定: `index.json`）
- `--port`, `-p`: 待ち受けポート（既定: 3000）

起動後:

- ブラウザで `http://127.0.0.1:3000/` を開くと検索フォームが表示されます。
- `GET /search?q=単語` で JSON の検索結果（URL と TF-IDF スコア）が返ります。

例:

```bash
curl "http://127.0.0.1:3000/search?q=rust"
```

## 構成

- `src/main.rs`: エントリ（clap で crawl / serve サブコマンド）
- `src/crawler.rs`: 同一サイト内の並列クロール
- `src/index.rs`: 転置インデックス（TF 付き）の構築・保存・読み込み・TF-IDF 検索
- `src/search.rs`: axum の検索ハンドラとトップページ（HTML）
- `src/tokenize.rs`: テキストの単語分割

## 参考

- [PLAN.md](../PLAN.md): 設計・手順の概要
