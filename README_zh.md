<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>面向 AI 编程代理的数字与日期溯源追踪</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#许可证"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#安装">安装</a> &bull;
  <a href="#快速开始">快速开始</a> &bull;
  <a href="#命令">命令</a> &bull;
  <a href="#支持的-ai-工具">支持的 AI 工具</a> &bull;
  <a href="#工作原理">工作原理</a> &bull;
  <a href="CONTRIBUTING.md">贡献</a>
</p>

<p align="center">
  <a href="README.md">English</a> &bull;
  <a href="README_fr.md">Francais</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Espanol</a> &bull;
  <a href="README_pt.md">Português</a>
</p>

---

RGT 为 AI 编程代理提供持久、可查询的记忆，记录它们从源文件读取或通过计算得到的每个数字和日期，然后**验证每个推导在数学上都是正确的**。单一 Rust 二进制，支持 13 种 AI 编程工具，钩子只记录数据，从不改写命令。

## RGT 的功能

代理会在变化的文件上推理，也可能在运算上出错。RGT 追踪每个数值的来源，并重新检查运算。

| 操作 | RGT 的作用 |
|------|-----------|
| `rgt record <file>` | 从文件中提取每个数字和日期，录入溯源图 |
| `rgt status` | 报告总计、活跃和过期的节点，即哪些值仍然可信 |
| `rgt query <id>` | 将某个值的谱系追溯到其源文件 |
| `rgt derive` | 在**记录之前**，先针对父节点验证代理计算出的值 |
| `rgt graph` | 导出依赖 DAG（文本、Mermaid 或 DOT 格式） |
| `rgt hook` | 通过各代理原生的钩子/插件机制进行被动采集 |

## 为什么溯源追踪很重要

RGT 不衡量节省，它防止静默错误。有两个失败模式驱动它：

1. **数据过期**：代理读取了文件，之后文件发生变化，但代理仍依据旧数字进行推理。RGT 将受影响的根节点标记为**过期**，并把过期状态级联到所有依赖它们的派生值上（`rgt status`）。
2. **运算错误**：代理计算 `revenue = price * quantity` 时出错。RGT 会基于数据库中的父值重新计算表达式，并在不匹配的推导进入图之前**拒绝**它（退出码 1）。

钩子**仅用于溯源采集**：它们记录 `(path, content)`，从不改写、过滤或阻塞代理的工具调用。

## 安装

### 快速安装（Linux/macOS）

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> 安装到 `~/.local/bin`。如需加入 PATH：
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # 或 ~/.bashrc
> ```

### Homebrew（tap）

```bash
brew install rafael-bianchi/rgt/rgt
```

> tap 的 formula 会随每次 GitHub release 更新。

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### 预编译二进制

从 [releases](https://github.com/rafael-bianchi/rgt/releases) 下载：
- macOS：`rgt-aarch64-apple-darwin.tar.gz`
- Linux：`rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows：`rgt-x86_64-pc-windows-msvc.zip`

> macOS 和 Linux 二进制随每次 release 发布；Windows 二进制由 release CI 流水线生成，待其针对某个 tag 运行后出现。

### 自我更新

```bash
rgt update          # 用最新 release 原子替换二进制
rgt update --check  # 仅检查是否有新版本，不安装
```

### 验证安装

```bash
rgt status   # 显示溯源图状态（全新仓库从 0 个节点开始）
```

## 快速开始

```bash
# 1. 为你的 AI 工具配置溯源钩子
rgt init -g                 # 自动检测所有已安装的支持代理
rgt init -g --agent copilot # 或指定一个：copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # 项目级代理使用项目规则文件
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo 使用规则文件

# 2. 重启你的 AI 工具，然后：
rgt record budget.csv       # 代理读取数据文件并记录其值
rgt status                  # 检查图
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # 代理记录一条已验证的推导
rgt query node_drv_Z        # 追踪派生值的谱系
rgt graph --format mermaid  # 导出依赖图
```

## 工作原理

```
  代理读取数据文件                          代理计算派生值
            |                                           |
            v                                           v
  rgt hook post (原生钩子/插件)              rgt derive --parents <ids>
            |                                           |
            v                                  （针对父节点验证）
  rgt record 提取 {path, content}                       |
            |                                           v
            v                                     运算正确？
  溯源图 ── rgt status ── 过期？ ── 否 ──> 值可信
            |                    |
            +------- 是 ----------> 节点 + 依赖项标记为过期
```

三种策略保证图的可靠性：

1. **采集**：原生钩子/插件将代理读取的每个文件通过 `rgt record` 录入，无需代理记得调用。
2. **验证**：每次 `rgt derive` 都是"信任但要验证"：RGT 从父值重新计算结果，在插入前拒绝错误值。
3. **过期**：源文件变化时，其节点（以及所有派生值）被标记为过期，从而可以提示代理重新读取。

## 命令

### 初始化与钩子
```bash
rgt init [-g] [--force] [--agent <name>]   # 配置钩子，检测已安装代理
rgt hook pre|post [--agent <name>]         # 从代理事件 JSON（stdin）被动采集
```

### 溯源
```bash
rgt record <file>                          # 提取并追踪文件中的值
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # 验证并记录一条推导
rgt verify --parents <ids> --operation <op> --result <val>
                                           # 验证派生值而不记录
rgt status [--stale-only] [--json]         # 图状态与过期情况
rgt query <node_id> [--json]               # 某个值的完整谱系
rgt graph [-f text|mermaid|dot]            # 导出依赖 DAG
```

操作类型：`EXPRESSION`（如 `a + b * c` 的公式）和 `DATE_DIFF`（日期运算，如 `date2 - date1`）。父变量映射为 `parent[0]=a, parent[1]=b, ...`。

## 支持的 AI 工具

RGT 使用各代理的原生机制，为 13 种 AI 编程工具配置溯源采集钩子：

| 工具 | 安装 | 方式 |
|------|------|------|
| **Claude Code** | `rgt init -g` | PreToolUse/PostToolUse shell 钩子（`settings.json`） |
| **Cursor** | `rgt init -g --agent cursor` | pre/postToolUse 钩子（`hooks.json`） |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | Copilot Chat 钩子（`github.copilot.chat.hooks`） |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | 指令文件（Copilot CLI 配置目录） |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | `pre_tool` 钩子（`hooks.toml`）+ 提示词 |
| **OpenCode** | `rgt init -g --agent opencode` | TypeScript 插件 |
| **Pi** | `rgt init --agent pi`（或 `-g`） | TypeScript 扩展 |
| **Hermes** | `rgt init --agent hermes` | Python 插件 + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | `AGENTS.md` 指令 |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

接受的 `--agent` 值：`claude-code`、`cursor`、`codex`、`windsurf`、`copilot`、`gemini`、`vibe`、`opencode`、`pi`、`hermes`、`cline`、`antigravity`、`kilocode`，以及别名 `claude`、`roo-code`、`kilo`。

## 数据与存储

RGT 将图存储在项目根目录的 `.rgt/store.db`（SQLite）中，由 `rgt init` 创建。正常运行期间没有外部服务、没有遥测、没有网络调用。

## 文档

- **[AGENTS.md](AGENTS.md)**：RGT 为 AI 代理安装的指令（如何记录与推导）
- **[CONTRIBUTING.md](CONTRIBUTING.md)**：贡献指南
- **[CHANGELOG.md](CHANGELOG.md)**：发布历史

## 致谢

RGT 的灵感来自 [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)，一个为 AI 编程代理压缩 shell 输出的高性能 CLI 代理。RGT 采用 RTK 的做法：单一二进制 + 原生、按代理定制的钩子集成，并覆盖其 13 个代理。RTK 过滤命令输出，而 RGT 追踪代理读取和推导内容的数字与日期溯源，并验证每个推导在数学上是否正确。

RGT 借助 [DeepSeek](https://www.deepseek.com/) AI 编码工具构建。

## 贡献

欢迎贡献！请在 [GitHub](https://github.com/rafael-bianchi/rgt) 上提交 issue 或 PR。

## 许可证

基于 [Apache License, Version 2.0](LICENSE) 授权。
