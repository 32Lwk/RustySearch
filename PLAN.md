# ミニ検索エンジンの作り方

Rust で「クローラー → インデックス → 検索」を実装する手順。

---

## 全体の流れ

```
[URLを指定] → [クローラー] → [HTML取得] → [テキスト抽出] → [インデックス構築]
                                                                    ↓
[ユーザーが「Rust」と検索] ← [検索API] ← [インデックスを参照]
```

---

## 使用する主な crate

| 役割 | crate | 用途 |
|------|--------|------|
| HTTP クライアント | `reqwest` | ページを取得（async） |
| HTML パース | `scraper` | タグ除去・リンク抽出 |
| 非同期ランタイム | `tokio` | async/await |
| Web サーバー | `axum` | 検索APIを提供 |
| シリアライズ | `serde` | インデックスの保存（後で） |

---

## Phase 1: 1 ページを取得してテキストを取り出す

**目標**: 1 つの URL を渡すと、そのページの本文テキストが取れる。

### 手順

1. **プロジェクト作成**
   ```bash
   cargo new mini-search-engine
   cd mini-search-engine
   ```

2. **Cargo.toml に依存を追加**
   ```toml
   [dependencies]
   reqwest = { version = "0.11", features = ["blocking"] }  # まずは blocking でも可
   scraper = "0.18"
   tokio = { version = "1", features = ["full"] }
   ```

3. **やること**
   - `reqwest::get(url)` で HTML を取得
   - `scraper::Html::parse_document(&html)` でパース
   - `Selector::parse("body").unwrap()` で body を選び、`.text()` でテキストだけ取り出す
   - リンク抽出は `Selector::parse("a[href]")` で `href` を取得

4. **確認**
   - `cargo run -- https://example.com` で example.com のテキストが表示されれば OK

---

## Phase 2: 同じサイト内を再帰的にクロール

**目標**: スタート URL から、同じドメイン内のページをたどって全部取る。

### 手順

1. **URL の正規化**
   - `url` crate を追加
   - 相対パス → 絶対 URL に変換（`base.join(relative)?`）
   - ドメインがスタート URL と同じかチェック

2. **クロールの流れ**
   - `Vec` または `HashSet` で「すでに訪れた URL」を管理
   - キュー（`VecDeque`）に「これから訪れる URL」を入れる
   - 1 ページ取得 → リンクを抽出 → 同じドメインかつ未訪問ならキューに追加
   - 深さ制限（例: 最大 3 階層）や最大ページ数（例: 50）を設けると安全

3. **データ構造の例**
   ```rust
   struct CrawlResult {
       url: String,
       title: String,
       body_text: String,
       links: Vec<String>,
   }
   ```

4. **確認**
   - 自分のブログや docs の URL を 1 つ渡して、複数ページ分の `CrawlResult` が得られれば OK

---

## Phase 3: 転置インデックスを構築

**目標**: 全ページのテキストから「単語 → その単語が含まれる URL のリスト」を作る。

### 手順

1. **テキスト → 単語に分割**
   - 空白・改行で split
   - 記号を除く（`trim_matches(|c: char| !c.is_alphanumeric())` など）
   - 小文字に正規化（`to_lowercase()`）して同じ単語を揃える

2. **転置インデックス**
   - 型の例: `HashMap<String, HashSet<String>>`
     - キー: 単語
     - 値: その単語が含まれる URL の集合（重複排除に HashSet）
   - 各 `CrawlResult` の `body_text` を単語に分割し、`url` を対応する単語の集合に追加

3. **確認**
   - Phase 2 の結果を渡してインデックスを組み、特定の単語で `get` すると URL のリストが返れば OK

---

## Phase 4: 検索 API（axum）で「?q=単語」に答える

**目標**: ブラウザや curl で `http://localhost:3000/search?q=Rust` とすると、ヒットした URL の JSON が返る。

### 手順

1. **依存追加**
   ```toml
   axum = { version = "0.7", features = ["json"] }
   tower_http = { version = "0.5", features = ["cors"] }  # 必要なら
   ```

2. **状態の持ち方**
   - インデックスをアプリで保持する
   - `axum::extract::State` でハンドラに渡す
   - 型例: `AppState { index: HashMap<String, HashSet<String>> }`

3. **ルート**
   - `GET /search?q=単語`
   - `q` を取って、インデックスで検索
   - ヒットした URL を Vec にして `Json(...)` で返す

4. **main の流れ**
   - Phase 2 でクロール → Phase 3 でインデックス構築（起動時に 1 回 or 別コマンドで事前構築）
   - そのインデックスを `State` にして `axum::Router::new().route("/search", get(search_handler))`
   - `axum::serve` で `localhost:3000` を listen

5. **確認**
   - `curl "http://localhost:3000/search?q=Rust"` で JSON が返れば OK

---

## Phase 5: 並列クロール（速度アップ）

**目標**: 複数ページを同時に取得してクロールを速くする。

### 手順

1. **reqwest を async に**
   - `reqwest::Client::new()` を共有
   - `client.get(url).send().await` で非同期取得

2. **並列のやり方**
   - キューから複数 URL を出し、`tokio::spawn` で並列に取得
   - セマフォ（`tokio::sync::Semaphore`）で同時リクエスト数を制限（例: 5）
   - 結果を `mpsc` や `JoinSet` で集約

3. **注意**
   - 同じ URL を二重にキューに入れない（`HashSet` で訪問済みを管理し、追加前にチェック）

---

## 以降の拡張（Phase 6 以降）

- **ランキング**: TF-IDF や BM25 でスコアを付け、ヒットした URL をスコア順に返す
- **永続化**: インデックスを `serde` で JSON/バイナリにしてファイルに保存し、起動時に読み込む
- **CLI**: `crawl --url https://...` と `serve` をサブコマンドで切り替え
- **フロント**: 簡単な HTML で検索フォームを作り、`/search` を呼ぶ

---

## ディレクトリ構成の例

```
mini-search-engine/
├── Cargo.toml
├── src/
│   ├── main.rs          # エントリ（crawl → index → serve）
│   ├── crawler.rs       # Phase 2 のクロール
│   ├── index.rs         # Phase 3 の転置インデックス
│   ├── search.rs        # Phase 4 の検索ハンドラ
│   └── tokenize.rs      # テキスト → 単語
└── PLAN.md              # このファイル
```

---

## まとめ

| Phase | やること | 学ぶ Rust のポイント |
|-------|----------|----------------------|
| 1 | 1 ページ取得・テキスト抽出 | 外部 crate、Result、文字列 |
| 2 | 同一サイト再帰クロール | コレクション、URL 処理、ループ |
| 3 | 転置インデックス | HashMap、HashSet、所有権 |
| 4 | axum で検索 API | 状態共有、非同期、HTTP |
| 5 | 並列クロール | tokio、spawn、セマフォ |

Phase 1 から順にやれば、少しずつ「検索エンジン」が形になっていく。
