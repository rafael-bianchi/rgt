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

> macOS と Linux のバイナリは各 release で公開されます。Windows バイナリは、tag に対して release CI パイプラインが実行された後に公開されます。

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
2. **検証**：すべての `rgt derive` は"信頼するが検証する"。RGT は親値から結果を再計算し、間違った値を挿入前に拒否します。
3. **古い状態の検出**：ソースファイルが変わると、そのノード（およびそこから派生するすべて）が古い状態とマークされ、エージェントに再読み込みを促せます。

## コマンド

### 初期化とフック
```bash
rgt init [-g] [--force] [--agent <name>]   # フックを設定し、インストール済みエージェントを検出
rgt hook pre|post [--agent <name>]         # エージェントのイベント JSON（stdin）から受動的に取得
```

### 来歴
```bash
rgt record <file>                          # ファイルから値を抽出して追跡
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # 導出を検証して記録
rgt verify --parents <ids> --operation <op> --result <val>
                                           # 記録せずに派生値を検証
rgt status [--stale-only] [--json]         # グラフの状態とデータの古さ
rgt query <node_id> [--json]               # 値の完全な系譜
rgt graph [-f text|mermaid|dot]            # 依存 DAG をエクスポート
```

操作：`EXPRESSION`（`a + b * c` のような式）と `DATE_DIFF`（`date2 - date1` のような日付演算）。親変数は `parent[0]=a, parent[1]=b, ...` と対応します。

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
