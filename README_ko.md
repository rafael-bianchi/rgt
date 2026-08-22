<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>AI 코딩 에이전트를 위한 숫자·날짜 출처 추적</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#라이선스"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#설치">설치</a> &bull;
  <a href="#빠른-시작">빠른 시작</a> &bull;
  <a href="#명령어">명령어</a> &bull;
  <a href="#지원-ai-도구">지원 AI 도구</a> &bull;
  <a href="#동작-원리">동작 원리</a> &bull;
  <a href="CONTRIBUTING.md">기여</a>
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

RGT는 AI 코딩 에이전트가 소스 파일에서 읽거나 계산으로 유도한 모든 숫자와 날짜를 영속적이고 검색 가능한 형태로 기억한 다음, **각 도출값이 수학적으로 올바른지 검증**합니다. 단일 Rust 바이너리, 13개 지원 AI 도구, 데이터만 기록하고 명령어를 절대 다시 쓰지 않는 훅을 갖추고 있습니다.

## RGT가 하는 일

에이전트는 변하는 파일을 기반으로 추론하고, 산수를 틀릴 수도 있습니다. RGT는 모든 숫자 값의 출처를 추적하고 계산을 다시 확인합니다.

| 작업 | RGT가 하는 일 |
|------|---------------|
| `rgt record <file>` | 파일에서 모든 숫자와 날짜를 추출하여 출처 그래프에 추가 |
| `rgt status` | 전체·활성·오래된 노드 수를 보고하여 어떤 값이 여전히 신뢰할 수 있는지 보여 줌 |
| `rgt derive` | 에이전트가 계산한 값을 **기록하기 전에** 부모 노드의 값을 기준으로 검증 |
| `rgt query <id>` | 값의 계보를 소스 파일까지 거슬러 추적 |
| `rgt graph` | 의존성 DAG 내보내기 (텍스트, Mermaid, DOT) |
| `rgt hook` | 각 에이전트의 네이티브 훅/플러그인 메커니즘을 통한 자동 수집 |

## 출처 추적이 중요한 이유

RGT는 절약을 측정하지 않습니다. 드러나지 않는 오류를 방지합니다. 주로 두 가지 오류 유형을 다룹니다.

1. **오래된 데이터**: 에이전트가 파일을 읽고, 이후 파일이 바뀌었는데도 에이전트는 옛 숫자로 추론을 계속합니다. RGT는 영향을 받는 루트 노드를 **오래된 상태**로 표시하고, 그에 의존하는 모든 도출값에 이 상태를 전파합니다(`rgt status`).
2. **잘못된 계산**: 에이전트가 `revenue = price * quantity` 계산을 잘못합니다. RGT는 데이터베이스의 부모 값으로 식을 다시 계산하고, 일치하지 않는 도출을 그래프에 들어가기 전에 **거부**합니다(종료 코드 1).

훅은 **출처 캡처 전용**입니다. `(path, content)`를 기록할 뿐 에이전트의 도구 호출을 다시 쓰거나, 필터링하거나, 차단하지 않습니다.

## 설치

### 빠른 설치 (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> `~/.local/bin`에 설치됩니다. 필요 시 PATH에 추가:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # 또는 ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> tap 공식(formula)은 GitHub 릴리스가 게시될 때마다 갱신됩니다.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### 사전 빌드된 바이너리

[releases](https://github.com/rafael-bianchi/rgt/releases)에서 다운로드:
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> macOS와 Linux 바이너리는 각 릴리스와 함께 게시됩니다. Windows 바이너리는 릴리스 CI 파이프라인이 태그에 대해 실행된 후 게시됩니다.

### 자동 업데이트

```bash
rgt update          # 최신 릴리스로 바이너리를 원자적으로 교체
rgt update --check  # 설치하지 않고 새 버전이 있는지만 확인
```

### 설치 확인

```bash
rgt status   # 출처 그래프 상태 표시 (새 저장소는 0개 노드로 시작)
```

## 빠른 시작

```bash
# 1. AI 도구에 출처 훅 구성
rgt init -g                 # 설치된 모든 지원 에이전트 자동 감지
rgt init -g --agent copilot # 또는 하나만 지정: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # 프로젝트 범위 에이전트는 프로젝트 규칙 파일 사용
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo는 규칙 파일 사용

# 2. AI 도구를 다시 시작한 후:
rgt record budget.csv       # 에이전트가 데이터 파일을 읽고 값을 기록
rgt status                  # 그래프 확인
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # 에이전트가 검증된 도출을 기록
rgt query node_drv_Z        # 도출값의 계보 추적
rgt graph --format mermaid  # 의존성 그래프 내보내기
```

## 동작 원리

```
  에이전트가 데이터 파일을 읽음                        에이전트가 도출값 계산
            |                                                  |
            v                                                  v
  rgt hook post (네이티브 훅/플러그인)              rgt derive --parents <ids>
            |                                                  |
            v                                  (부모 값을 기준으로 검증)
  rgt record가 {path, content} 추출                          |
            |                                                  v
            v                                        계산이 올바른가?
  출처 그래프 ── rgt status ── 오래됨? ── 아니오 ──> 값을 신뢰
            |                                    |
            +--------------- 예 ---------------->  노드와 종속 노드를 오래된 상태로 표시
```

그래프를 신뢰할 수 있게 만드는 세 가지 전략:

1. **캡처**: 네이티브 훅/플러그인이 에이전트가 읽는 모든 파일을 `rgt record`로 보내므로, 에이전트가 신경 쓰지 않아도 값이 그래프에 들어갑니다.
2. **검증**: 모든 `rgt derive`는 "신뢰하되 검증한다"는 원칙을 따릅니다. RGT는 부모 값에서 결과를 다시 계산하고 잘못된 값을 삽입 전에 거부합니다.
3. **최신성**: 소스 파일이 바뀌면 그 노드(및 그로부터 도출된 모든 값)가 오래된 상태로 표시되어 에이전트가 다시 읽도록 안내할 수 있습니다.

## 명령어

### 초기화 및 훅
```bash
rgt init [-g] [--force] [--agent <name>]   # 훅 구성, 설치된 에이전트 감지
rgt hook pre|post [--agent <name>]         # 에이전트 이벤트 JSON(stdin)에서 자동 수집
```

### 출처 추적
```bash
rgt record <file>                          # 파일에서 값을 추출하여 추적
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # 도출을 검증하고 기록
rgt verify --parents <ids> --operation <op> --result <val>
                                           # 기록하지 않고 도출값 검증
rgt status [--stale-only] [--json]         # 그래프 상태와 최신성
rgt query <node_id> [--json]               # 값의 전체 계보
rgt graph [-f text|mermaid|dot]            # 의존성 DAG 내보내기
```

연산: `EXPRESSION`(`a + b * c` 같은 수식)과 `DATE_DIFF`(`date2 - date1` 같은 날짜 연산). 부모 변수는 `parent[0]=a, parent[1]=b, ...`로 매핑됩니다.

## 지원 AI 도구

RGT는 13개 AI 코딩 도구에 대해 각 에이전트의 네이티브 메커니즘으로 출처 캡처 훅을 구성합니다:

| 도구 | 설치 | 방식 |
|------|------|------|
| **Claude Code** | `rgt init -g` | PreToolUse/PostToolUse 셸 훅(`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | pre/postToolUse 훅(`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | Copilot Chat 훅(`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | 지시 파일(Copilot CLI 설정 디렉터리) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | `pre_tool` 훅(`hooks.toml`) + 프롬프트 |
| **OpenCode** | `rgt init -g --agent opencode` | TypeScript 플러그인 |
| **Pi** | `rgt init --agent pi`(또는 `-g`) | TypeScript 확장 |
| **Hermes** | `rgt init --agent hermes` | Python 플러그인 + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | `AGENTS.md` 지시 |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

허용되는 `--agent` 값: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, 그리고 별칭 `claude`, `roo-code`, `kilo`.

## 데이터 및 저장소

RGT는 그래프를 프로젝트 루트의 `.rgt/store.db`(SQLite)에 저장하며 `rgt init`이 생성합니다. 정상 작동 중에는 외부 서비스, 텔레메트리, 네트워크 호출이 없습니다.

## 문서

- **[AGENTS.md](AGENTS.md)**: RGT가 AI 에이전트용으로 설치하는 지시(기록 및 도출 방법)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: 기여 가이드
- **[CHANGELOG.md](CHANGELOG.md)**: 릴리스 기록

## 감사의 말

RGT는 AI 코딩 에이전트를 위해 셸 출력을 압축하는 고성능 CLI 프록시인 [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)에서 영감을 받았습니다. RGT는 RTK의 접근 방식(단일 바이너리 + 네이티브 에이전트별 훅 통합)을 따르고 동일하게 13개 에이전트를 지원합니다. RTK가 명령 출력을 필터링하는 반면, RGT는 에이전트가 읽고 도출하는 내용의 숫자·날짜 출처를 추적하고 각 도출이 수학적으로 올바른지 검증합니다.

RGT는 [DeepSeek](https://www.deepseek.com/) AI 코딩 도구의 도움을 받아 개발되었습니다.

## 기여

기여를 환영합니다! [GitHub](https://github.com/rafael-bianchi/rgt)에서 issue 또는 PR을 열어주세요.

## 라이선스

[Apache License, Version 2.0](LICENSE)에 따라 라이선스가 부여됩니다.
