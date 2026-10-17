<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>AI コーディングエージェント向けの数値・日付の来歴追跡</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#ライセンス"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#インストール">インストール</a> &bull;
  <a href="#クイックスタート">クイックスタート</a> &bull;
  <a href="#コマンド">コマンド</a> &bull;
  <a href="#対応-ai-ツール">対応 AI ツール</a> &bull;
  <a href="#動作の仕組み">動作の仕組み</a> &bull;
  <a href="CONTRIBUTING.md">コントリビュート</a>
</p>

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Français</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Español</a> &bull;
  <a href="README_pt.md">Português</a>
</p>

---

RGT は、AI コーディングエージェントがソースファイルから読み取った、または計算で導出したすべての数値と日付を永続的で検索可能な形で記憶し、**それぞれの導出が数学的に正しいかを検証**します。単一の Rust バイナリ、13 の対応 AI ツール、データのみを記録しコマンドを書き換えないフックを備えています。

## RGT の機能

エージェントは変化するファイルに対して推論し、計算を誤ることもあります。RGT はすべての数値の来歴を追跡し、計算を再チェックします。

| 操作 | RGT の動作 |
|------|-----------|
| `rgt record <file>` | ファイルからすべての数値と日付を抽出して来歴グラフに追加 |
| `rgt status` | 総数・アクティブ・古いノードを報告。どの値がまだ信頼できるかを示す |
| `rgt derive` | エージェントが計算した値を**記録する前に**親ノードの値に基づいて検証 |
| `rgt query <id>` | 値の系譜をソースファイルまで遡る |
| `rgt graph` | 依存 DAG をエクスポート（テキスト、Mermaid、DOT） |
| `rgt hook` | 各エージェントのネイティブなフック/プラグイン機構による受動的な取得 |

## 来歴追跡が重要な理由

RGT は節約を測定するのではなく、検知されないエラーを防ぎます。主に 2 つの失敗パターンを想定しています。

1. **古いデータ**：エージェントがファイルを読み、その後ファイルが変わっても、エージェントは古い数値に基づいて推論を続けます。RGT は影響を受けるルートノードを**古い状態**とマークし、それに依存するすべての派生値へその状態を伝播させます（`rgt status`）。
2. **計算ミス**：エージェントが `revenue = price * quantity` を計算して誤る場合。RGT はデータベース内の親値から式を再計算し、一致しない導出をグラフに入る前に**拒否**します（終了コード 1）。

フックは**来歴の取得のみ**です。`(path, content)` を記録するだけで、エージェントのツール呼び出しを書き換えたり、フィルタリングしたり、ブロックしたりすることはありません。

## インストール

### クイックインストール（Linux/macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> `~/.local/bin` にインストールされます。必要に応じて PATH に追加：
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # または ~/.bashrc
> ```

### Homebrew（tap）

```bash
brew install rafael-bianchi/rgt/rgt
```

> tap の formula は各 GitHub release で更新されます。

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### プレビルドバイナリ

[releases](https://github.com/rafael-bianchi/rgt/releases) からダウンロード：
- macOS：`rgt-aarch64-apple-darwin.tar.gz`
- Linux：`rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows：`rgt-x86_64-pc-windows-msvc.zip`

> macOS、Linux、Windows のプリビルドバイナリは各 release で公開されます。

### 自己更新

```bash
rgt update          # 最新リリースでバイナリを原子的に置き換え
rgt update --check  # インストールせずに新しいバージョンの有無だけを確認
```

### インストールの確認

```bash
rgt status   # 来歴グラフの状態を表示（新規ストアは 0 ノードから始まる）
```

## クイックスタート

```bash
# 1. AI ツール用に来歴フックを設定
rgt init -g                 # インストール済みの対応エージェントをすべて自動検出
rgt init -g --agent copilot # または 1 つを指定：copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # プロジェクトスコープのエージェントはプロジェクト規則ファイルを使用
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo は規則ファイルを使用

# 2. AI ツールを再起動し、次を実行：
rgt record budget.csv       # エージェントがデータファイルを読み、値を記録
rgt status                  # グラフを確認
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # エージェントが検証済みの導出を記録
rgt query node_drv_Z        # 派生値の系譜を追跡
rgt graph --format mermaid  # 依存グラフをエクスポート
rgt graph --format ttl      # PROV-O Turtle でエクスポート
```

## 動作の仕組み

```
  エージェントがデータファイルを読む                    エージェントが派生値を計算
            |                                                   |
            v                                                   v
  rgt hook post（ネイティブフック/プラグイン）      rgt derive --parents <ids>
            |                                                   |
            v                                         （親に対して検証）
  rgt record が {path, content} を抽出                        |
            |                                                   v
            v                                             計算は正しい？
  来歴グラフ ── rgt status ── 古い？ ── いいえ ──> 値は信頼できる
            |                                   |
            +--------------- はい ---------------> ノードとそれに依存するノードを古い状態とマーク
```

グラフを信頼に足るものにする 3 つの戦略：

1. **取得**：ネイティブフック/プラグインが、エージェントが読むすべてのファイルを `rgt record` に通すため、エージェントが意識しなくても値がグラフに入ります。

> **非テキストソース（PDF・Excel・画像）**：RGT のパッシブフックはバイナリファイルの中を見ることができません。PDF/画像の解釈はモデル内部で行われ、ローカルのテキストファイルとしては存在しません。RGT がエージェントに書き込む指示では、`echo "金額: 1234.56" | rgt record --stdin`（または中間の `.txt`/`.md` ファイル）でこうした値を明示的に報告するよう求めています。プレーンテキスト・CSV・JSON・Markdown は自動取得されるため何もする必要はありません。バイナリファイルに対する `rgt record` は、この経路を示す明確な「サポートされていないファイル形式」エラーを返します。
2. **検証**：すべての `rgt derive` は"信頼するが検証する"。RGT は親値から結果を再計算し、間違った値を挿入前に拒否します。
3. **古い状態の検出**：ソースファイルが変わると、そのノード（およびそこから派生するすべて）が古い状態とマークされ、エージェントに再読み込みを促せます。

## コマンド

### 初期化とフック
```bash
rgt init [-g] [--force] [--agent <name>] [--number-format <us|eu|auto>]  # フックを設定し、インストール済みエージェントを検出
rgt hook pre|post [--agent <name>]         # エージェントのイベント JSON（stdin）から受動的に取得
```

`--number-format` は、パッシブフック経路で数値の解析に使うロケール（`.`/`,` 区切りと符号）を永続化します。有効な値：`us`（小数点 `.`、桁区切り `,`）、`eu`（小数点 `,`、桁区切り `.`）、または `auto`（ファイルごとに推定 — デフォルト）。

`rgt init` は既存の設定を決して破壊しません。RGT のフックエントリをエージェント設定に加算的にマージします — JSON/JSONC ファイル（コメントと末尾のカンマは許容・保持）、TOML 設定、ルールファイルは、他のフック・キーバインド・プラグイン・メモをすべてバイト単位で保持します。`--force` なしでの再実行は何もしません。`--force` は RGT 専用の区切りブロックだけを更新します。設定ファイルを解析・安全にマージできない場合は、そのエージェントが回復可能なエラーと非ゼロ終了コードで報告されます — 他のエージェントは引き続き設定され、既存ファイルの書き換え前には `<ファイル>.rgt.bak` バックアップが保持されます。
終了コード：`0` = 成功、`1` = 少なくとも 1 つのエージェントで失敗、`2` = 不正な `--agent` 名。

`rgt init` は、実行中のバイナリから解決した**絶対パス**で CLI を呼び出すフックコマンドを書き込むため、AI ツールが起動するシェルの PATH に依存しません。バイナリを移動した場合（例：パッケージマネージャーのアップグレードで別のパスにインストールされる場合）は、`rgt init` を再実行して書き込まれたパスを更新してください。

### Doctor

```bash
rgt doctor   # フックのインストールと取得の健全性を診断
```

`rgt doctor` は `brew doctor` スタイルの診断です。正常または参考警告の場合は終了 `0`、明確な問題（例：ストアが初期化されていない）の場合は `1` です。フックがインストールされているのに値が一度も記録されていない場合、取得が有効でない可能性があると警告し、AI ツールが使うシェルコンテキストで `rgt` が解決可能か確認するよう促します（フックのサブプロセスはフェイルオープンなので、PATH が壊れていると何も記録されずに黙って失敗します）。

### 更新
```bash
rgt update [--check] [--yes] [--version <tag>] [--skip-checksum] [--skip-attestation]
```

`rgt update` は**検証済み**のバイナリのみをインストールします：すべてのダウンロードはリリースの `checksums.txt`（SHA-256）**と** GitHub Artifact Attestation の両方に対して検査されます — リリース自身のアセットとは独立に発行される信頼の基点です（組み込みの Sigstore 検証、`gh` 不要）。どちらかの検証を欠くリリースは非ゼロ終了で拒否されます。`--skip-checksum` / `--skip-attestation` は明示的で、非安全と文書化されたオプトアウトであり、デフォルトではありません。バージョン比較にはセマンティックバージョニングを使用します：古いか等しいタグは決してインストールされません（黙ってのダウングレードなし）。`--version <tag>` は要求されたタグがインストール済みバージョンより古い場合に警告します。

### 来歴
```bash
rgt record <file> [--number-format <us|eu|auto>]   # ファイルから値を抽出して追跡
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                            # 導出を検証して記録
rgt verify --parents <ids> --operation <op> --result <val>
                                            # 記録せずに派生値を検証
rgt status [--stale-only] [--json]         # グラフの状態とデータの古さ
rgt query <node_id> [--json]               # 値の完全な系譜
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths] # 依存グラフを出力
rgt gc [--vacuum]                          # 廃止ノード（古く、依存先のないもの）を削除
rgt mcp                                   # stdio 経由で Model Context Protocol サーバーを実行
```

`rgt record --number-format` は 1 回の呼び出しで永続化された形式を上書きします。それ以外は（`rgt init` の）永続化設定が使われ、`auto` にフォールバックします。数値解析はロケール対応です：先頭の `-` と会計括弧 `(N)` は値の一部、有効な形式に応じて桁区切り/小数点が尊重され、その形式で解析できない値は（警告付きで）スキップされ、誤ったノードに分割されることはありません。

#### Turtle 来歴エクスポート

`rgt graph --format ttl` は、PROV-O のエンティティ、アクティビティ、利用関係、エージェントを用いて決定論的な Turtle を出力します。プロジェクトスコープの値 IRI により、独立したストアからの出力を RDF 統合した際にもローカルノード ID の衝突を防ぎます。元のノード ID は `rgt:localNodeId` として保持されます。

デフォルトではソースドキュメントはパスを含まない IRI を持ちます。ファイルが存在する場合、安全に検証されたプロジェクト相対パスが付与されます。`--include-absolute-paths` を指定すると絶対パスが出力されますが、ユーザー名やローカルパス情報が含まれる場合があります。

Turtle エクスポートは 10,000 個の記録値、30,000 本の導出エッジ、64 MiB、1 秒の実行期限に制限されています。

`rgt gc` は**廃止ノード** — 古いとマークされ、何も派生していない値 — を削除し、削除数を報告します。古くないノードや依存されているものには一切触れません。`rgt gc --vacuum` はさらに SQLite の `VACUUM` で解放されたディスクページを回収します（オプション；デフォルト実行は行の削除のみ）。

### 期間の計算と表示

操作：`EXPRESSION`、`DATE_DIFF`、`DURATION_SUM`、`DURATION_AVG`。`DATE_DIFF` は順序付きの2つの日付から経過時間を自動計算します（`date2 - date1`）。負の値や秒未満の端数は拒否されます。合計と平均は、重複しない2〜26個の Duration を受け取ります。結果は正確な整数秒として保存され、端数秒を要する計算は拒否されます。親変数は `parent[0]=a, parent[1]=b, ...` と対応します（最大 26、`a`–`z`）。

表示単位は `seconds`、`minutes`、`hours`、`days`、`weeks` です。暦上の月と年は長さが変わるためサポートしません。`--result-unit` は入力した値の単位を指定し、`--unit` は表示だけを変えます。どちらの単位もノードのメタデータとしては永続化されません：

```bash
rgt derive --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days --unit hours
rgt derive --parents <duration1>,<duration2> --operation DURATION_SUM --unit minutes
rgt verify --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days
rgt query <duration_id> --unit days [--json]
```

クエリは既存の `value` を保持し、要求したノードだけに正確な秒数を文字列で示す `duration_seconds` と `selected_unit_display` を追加します。循環小数の変換には近似の印が付きますが、秒数は常に正確です。`EXPRESSION` は日付を Unix タイムスタンプ秒、Duration を経過秒として扱い、Number を返して警告を出します。

`--operation` は大文字小文字を区別します（`EXPRESSION`/`DATE_DIFF`/`DURATION_SUM`/`DURATION_AVG`）；それ以外は終了コード 2 で拒否され、検証が黙ってスキップされることはありません。`--expression` は 4096 バイトと括弧の入れ子 256 レベルに制限されます（超過はクラッシュではなく終了コード 2 で拒否）。式は組み込み関数を無効化して評価されるため、親変数と `+ - * / % ^`、括弧だけが使え、評価は決定的です。

### MCP サーバー

`rgt mcp` は RGT を stdio 経由の [Model Context Protocol](https://modelcontextprotocol.io/) サーバーとして実行し、同じ操作を MCP ツールとして公開します。エージェントごとのフック形式に代わる単一のドキュメント化された統合手段として機能します（Claude Code, Cursor, VS Code, ChatGPT 等に対応）：

| ツール | 説明 |
|--------|------|
| `query_provenance` | 値の完全な系譜を追跡 |
| `get_status` | グラフ状態（全ノード/有効/無効）を取得 |
| `record_values` | ファイル内容から値を抽出・記録 |
| `verify_only` | 記録せずに導出の正確性を検証 |
| `derive_and_verify` | 導出を検証し、合格時に記録 |

プロジェクト設定で MCP サーバーを起動するように設定します（例）：

```json
{
  "mcpServers": {
    "rgt": { "command": "rgt", "args": ["mcp"] }
  }
}
```

サーバーは起動元のプロジェクトディレクトリで動作します。無効な入力はツールエラーとして返され、サーバープロセスがクラッシュすることはありません。

## 対応 AI ツール

RGT は 13 種類の AI コーディングツールに対し、各エージェントのネイティブ機構を使って来歴取得フックを設定します：

| ツール | インストール | 方法 |
|--------|--------------|------|
| **Claude Code** | `rgt init -g` | PreToolUse/PostToolUse シェルフック（`settings.json`） |
| **Cursor** | `rgt init -g --agent cursor` | pre/postToolUse フック（`hooks.json`） |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | Copilot Chat フック（`github.copilot.chat.hooks`） |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | 指示ファイル（Copilot CLI 設定ディレクトリ） |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | `pre_tool` フック（`hooks.toml`）+ プロンプト |
| **OpenCode** | `rgt init -g --agent opencode` | TypeScript プラグイン |
| **Pi** | `rgt init --agent pi`（または `-g`） | TypeScript 拡張 |
| **Hermes** | `rgt init --agent hermes` | Python プラグイン + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | `AGENTS.md` 指示 |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

指定可能な `--agent` の値：`claude-code`、`cursor`、`codex`、`windsurf`、`copilot`、`gemini`、`vibe`、`opencode`、`pi`、`hermes`、`cline`、`antigravity`、`kilocode`、およびエイリアス `claude`、`roo-code`、`kilo`。

## データとストレージ

RGT はグラフをプロジェクトルートの `.rgt/store.db`（SQLite）に保存します。`rgt init` が作成します。通常の運用中は、外部サービスやテレメトリを使用せず、ネットワーク呼び出しも行いません。

## ドキュメント

- **[AGENTS.md](AGENTS.md)**：RGT が AI エージェント用にインストールする指示（記録と導出の方法）
- **[CONTRIBUTING.md](CONTRIBUTING.md)**：コントリビューションガイド
- **[CHANGELOG.md](CHANGELOG.md)**：リリース履歴

## 謝辞

RGT は、AI コーディングエージェント向けに shell 出力を圧縮する高性能 CLI プロキシ [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk) に触発されました。RGT は RTK のアプローチ（単一バイナリ＋ネイティブでエージェント固有のフック統合）に従い、13 エージェントのカバレッジを踏襲しています。RTK がコマンド出力をフィルタリングするのに対し、RGT はエージェントが読み取り・導出する内容の数値・日付の来歴を追跡し、各導出が数学的に正しいかを検証します。

RGT は [DeepSeek](https://www.deepseek.com/) の AI コーディングツールの支援を受けて開発されました。

## コントリビュート

コントリビューションを歓迎します。[GitHub](https://github.com/rafael-bianchi/rgt) で issue または PR を開いてください。

## ライセンス

[Apache License, Version 2.0](LICENSE) に基づいてライセンスされています。
