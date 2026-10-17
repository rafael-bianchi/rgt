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
  <a href="README_fr.md">Français</a> &bull;
  <a href="README_zh.md">中文</a> &bull;
  <a href="README_ja.md">日本語</a> &bull;
  <a href="README_ko.md">한국어</a> &bull;
  <a href="README_es.md">Español</a> &bull;
  <a href="README_pt.md">Português</a>
</p>

---

RGT 为 AI 编程代理提供持久、可查询的记忆，记录它们从源文件读取或通过计算得到的每个数字和日期，然后**验证每个推导在数学上都是正确的**。单一 Rust 二进制，支持 13 种 AI 编程工具，钩子只记录数据，从不改写命令。

## RGT 的功能

代理会在变化的文件上推理，也可能在运算上出错。RGT 追踪每个数值的来源，并重新检查运算。

| 操作 | RGT 的作用 |
|------|-----------|
| `rgt record <file>` | 从文件中提取每个数字和日期，录入溯源图 |
| `rgt status` | 报告节点总数以及有效和过期节点数，并指出哪些值仍然可信 |
| `rgt query <id>` | 将某个值的谱系追溯到其源文件 |
| `rgt derive` | 在**记录之前**，先根据父节点的值验证代理计算出的结果 |
| `rgt graph` | 导出依赖 DAG（文本、Mermaid 或 DOT 格式） |
| `rgt hook` | 通过各代理原生的钩子/插件机制进行被动采集 |

## 为什么溯源追踪很重要

RGT 不衡量节省，它防止静默错误。它主要针对两类故障：

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

> macOS、Linux 和 Windows 的预编译二进制随每次 release 发布。

### 自我更新

```bash
rgt update          # 用最新 release 原子替换二进制
rgt update --check  # 仅检查是否有新版本，不安装
```

### 验证安装

```bash
rgt status   # 显示溯源图状态（全新的存储从 0 个节点开始）
```

## 快速开始

```bash
# 1. 为你的 AI 工具配置溯源钩子
rgt init -g                 # 自动检测所有已安装且受支持的代理
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
rgt graph --format ttl      # 导出 PROV-O Turtle
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
  溯源图 ── rgt status ── 过期？ ── 否 ──> 值可被信任
            |                    |
            +------- 是 ----------> 该节点及其下游节点标记为过期
```

三种策略保证图的可靠性：

1. **采集**：原生钩子/插件将代理读取的每个文件通过 `rgt record` 录入，无需代理记得调用。

> **非文本来源（PDF、Excel、图片）**：RGT 的被动钩子无法查看二进制文件内部——PDF/图片的解读发生在模型内部，从不以本地文本文件的形式存在。RGT 写入代理的指令要求其通过 `echo "金额: 1234.56" | rgt record --stdin`（或中间 `.txt`/`.md` 文件）显式上报此类值。纯文本/CSV/JSON/Markdown 文件无需任何操作——它们会被自动采集。对二进制文件执行 `rgt record` 会返回一条明确的"不支持的文件格式"错误，说明这一路径。
2. **验证**：每次 `rgt derive` 都是"信任但要验证"：RGT 从父值重新计算结果，在插入前拒绝错误值。
3. **过期**：源文件变化时，其节点（以及所有派生值）被标记为过期，从而可以提示代理重新读取。

## 命令

### 初始化与钩子
```bash
rgt init [-g] [--force] [--agent <name>] [--number-format <us|eu|auto>]  # 配置钩子，检测已安装代理
rgt hook pre|post [--agent <name>]         # 从代理事件 JSON（stdin）被动采集
```

`--number-format` 为被动钩子路径持久化解析数字所用的区域设置（`.`/`,` 分隔符与符号）。有效值：`us`（小数点 `.`，千分位 `,`）、`eu`（小数点 `,`，千分位 `.`）或 `auto`（按文件推断——默认值）。

`rgt init` **绝不会破坏你现有的配置**。它以追加方式将 RGT 的钩子条目合并进代理配置——JSON/JSONC 文件（容忍并保留注释和尾随逗号）、TOML 配置和规则文件会逐字节保留其他所有钩子、按键绑定、插件和备注。未加 `--force` 重新运行是无操作；`--force` 仅刷新 RGT 自己的分隔块。如果某个配置文件无法解析或安全合并，该代理会以可恢复的错误和非零退出码报告——其他代理仍会被配置，且在重写任何现有文件之前会保留 `<文件>.rgt.bak` 备份。
退出码：`0` = 成功，`1` = 至少一个代理失败，`2` = 无效的 `--agent` 名称。

`rgt init` 写入的钩子命令通过**绝对路径**（从正在运行的二进制解析）调用 CLI，因此采集不依赖 AI 工具所启动 shell 的 PATH。如果移动了二进制（例如包管理器升级安装到新路径），请重新运行 `rgt init` 以刷新写入的路径。

### Doctor

```bash
rgt doctor   # 诊断钩子安装与采集健康状况
```

`rgt doctor` 是 `brew doctor` 风格的诊断工具。健康或仅有建议性警告时退出 `0`，存在明确问题（如存储未初始化）时退出 `1`。当钩子已安装但从未记录过任何值时，它会警告采集可能未激活，并提示你验证在 AI 工具使用的 shell 上下文中能否解析 `rgt`（钩子子进程失败即放行，因此 PATH 损坏时会在静默中不记录任何内容）。

### 更新
```bash
rgt update [--check] [--yes] [--version <tag>] [--skip-checksum] [--skip-attestation]
```

`rgt update` 只安装**已验证**的二进制：每次下载都会对照发布版的 `checksums.txt`（SHA-256）**以及**其 GitHub Artifact Attestation 进行检查——这是独立于发布版自身资产签发的信任锚点（内置 Sigstore 验证，无需 `gh`）。缺少任一验证的发布版会被以非零退出码拒绝；`--skip-checksum` / `--skip-attestation` 是显式、被记录为不安全的退出开关，绝非常规默认。版本比较使用语义化版本：相同或更旧的标签绝不会被安装（不会静默降级），`--version <tag>` 在请求的标签旧于已安装版本时会发出警告。

### 溯源
```bash
rgt record <file> [--number-format <us|eu|auto>]   # 提取并追踪文件中的值
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                            # 验证并记录一条推导
rgt verify --parents <ids> --operation <op> --result <val>
                                            # 验证派生值而不记录
rgt status [--stale-only] [--json]         # 图状态与过期情况
rgt query <node_id> [--json]               # 某个值的完整谱系
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths] # 导出依赖图
rgt gc [--vacuum]                          # 移除过期（陈旧且无派生的）节点
rgt mcp                                   # 通过 stdio 运行 Model Context Protocol 服务器
```

`rgt record --number-format` 仅对单次调用覆盖持久化格式；否则使用（来自 `rgt init` 的）持久化设置，并回退到 `auto`。数字解析是区域感知的：前导 `-` 和会计括号 `(N)` 属于值的一部分，按当前格式尊重千分位/小数点分隔符，而无法在该格式下解析的值会被跳过（带警告），而不是被拆分成错误的节点。

#### Turtle 溯源导出

`rgt graph --format ttl` 使用 PROV-O 实体、活动、用法和编程工具代理写入确定性的 Turtle。项目范围的值 IRI 可在通过 RDF 联合合并导出结果时，保持来自独立存储的匹配本地节点 ID 相互区分；复制存储可保留其项目标识。原始节点 ID 保留为 `rgt:localNodeId`。

默认情况下，源文档具有无路径的 IRI。当文件可用时，会包含一个经过安全验证的项目相对路径；外部、缺失或无法分类的路径则没有路径字面量。可选的 `--include-absolute-paths` 标志会添加可用的绝对源路径，这可能会暴露用户名或本地目录名。

Turtle 导出受限于 10,000 个记录值、30,000 条推导边、64 MiB 生成的 UTF-8 和 1 秒的命令截止时间。

`rgt gc` 移除**过期节点**——被标记为过期且没有任何派生的值——并报告移除了多少个。它绝不会触碰非过期节点或仍被依赖的内容。`rgt gc --vacuum` 额外用 SQLite `VACUUM` 回收释放的磁盘页（可选；默认运行仅删除行）。

### 时长计算与显示

操作类型：`EXPRESSION`、`DATE_DIFF`、`DURATION_SUM` 和 `DURATION_AVG`。`DATE_DIFF` 会自动计算两个有序日期之间的经过时间（`date2 - date1`）。负时间间隔或包含秒分数的间隔会被拒绝。求和和平均接受 2 到 26 个互不重复的 Duration。结果以精确的整数秒存储；需要秒分数的平均值或断言会被拒绝。父变量映射为 `parent[0]=a, parent[1]=b, ...`（最多 26 个，`a`–`z`）。

固定显示单位为 `seconds`、`minutes`、`hours`、`days` 和 `weeks`。日历月和年长度不固定，因此不支持。`--result-unit` 仅修饰断言单位，`--unit` 仅选择显示单位。这两个单位都不会作为节点元数据持久化：

```bash
rgt derive --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days --unit hours
rgt derive --parents <duration1>,<duration2> --operation DURATION_SUM --unit minutes
rgt verify --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days
rgt query <duration_id> --unit days [--json]
```

查询会保留现有 `value`，并且只为请求的节点添加以十进制字符串表示的精确秒数 `duration_seconds` 和 `selected_unit_display`。循环小数转换会标记为近似值，但精确秒数仍然可用。携带 `--unit` 查询日期或数字会被拒绝。`EXPRESSION` 带 Date 或 Duration 父节点时返回 Number 并输出警告。

`--operation` 区分大小写（`EXPRESSION`/`DATE_DIFF`/`DURATION_SUM`/`DURATION_AVG`）；其他值一律以退出码 2 拒绝——验证绝不会被静默跳过。`--expression` 限制为 4096 字节和 256 层括号嵌套（超出限制的输入以退出码 2 拒绝，绝不会崩溃）。表达式以内置函数禁用方式求值——只有你的父变量和 `+ - * / % ^` 及括号可用，因此求值是确定性的。

### MCP 服务器

`rgt mcp` 将 RGT 作为基于 stdio 的 [Model Context Protocol](https://modelcontextprotocol.io/) 服务器运行，将相同的操作作为标准 MCP 工具公开，提供单一的文档化集成界面替代各代理专用的钩子格式（支持 Claude Code, Cursor, VS Code, ChatGPT 等）：

| 工具 | 功能说明 |
|------|----------|
| `query_provenance` | 追溯某个值的完整谱系 |
| `get_status` | 获取图状态（总节点数/活跃数/陈旧数） |
| `record_values` | 从文件内容中提取并记录值 |
| `verify_only` | 验证推导计算而不记录 |
| `derive_and_verify` | 验证推导并在通过时记录结果 |

在你的项目目录中配置 MCP 客户端将其作为 stdio 子进程启动，例如：

```json
{
  "mcpServers": {
    "rgt": { "command": "rgt", "args": ["mcp"] }
  }
}
```

服务器从启动它的项目目录运行。任何无效输入都会作为工具错误返回给客户端——服务器进程绝不会因工具失败而崩溃。

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

RGT 的灵感来自 [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)，一个为 AI 编程代理压缩 shell 输出的高性能 CLI 代理。RGT 采用 RTK 的做法：单一二进制 + 原生、按代理定制的钩子集成，并同样支持 13 种代理。RTK 过滤命令输出，而 RGT 追踪代理读取和推导内容的数字与日期溯源，并验证每个推导在数学上是否正确。

RGT 借助 [DeepSeek](https://www.deepseek.com/) AI 编程工具构建。

## 贡献

欢迎贡献！请在 [GitHub](https://github.com/rafael-bianchi/rgt) 上提交 issue 或 PR。

## 许可证

基于 [Apache License, Version 2.0](LICENSE) 授权。
