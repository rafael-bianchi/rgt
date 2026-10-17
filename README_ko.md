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

> macOS, Linux, Windows용 사전 빌드 바이너리는 각 릴리스와 함께 게시됩니다.

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
rgt graph --format ttl      # PROV-O Turtle로 내보내기
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

> **비텍스트 소스(PDF·Excel·이미지)**: RGT의 수동 훅은 바이너리 파일 내부를 볼 수 없습니다. PDF/이미지 해석은 모델 내부에서 이루어지며 로컬 텍스트 파일로는 존재하지 않습니다. RGT가 에이전트에 작성하는 지시는 `echo "금액: 1234.56" | rgt record --stdin`(또는 중간 `.txt`/`.md` 파일)으로 이러한 값을 명시적으로 보고하도록 요구합니다. 일반 텍스트·CSV·JSON·Markdown 파일은 자동 캡처되므로 아무 조치도 필요 없습니다. 바이너리 파일에 대한 `rgt record`는 이 경로를 안내하는 명확한 "지원되지 않는 파일 형식" 오류를 반환합니다.
2. **검증**: 모든 `rgt derive`는 "신뢰하되 검증한다"는 원칙을 따릅니다. RGT는 부모 값에서 결과를 다시 계산하고 잘못된 값을 삽입 전에 거부합니다.
3. **최신성**: 소스 파일이 바뀌면 그 노드(및 그로부터 도출된 모든 값)가 오래된 상태로 표시되어 에이전트가 다시 읽도록 안내할 수 있습니다.

## 명령어

### 초기화 및 훅
```bash
rgt init [-g] [--force] [--agent <name>] [--number-format <us|eu|auto>]  # 훅 구성, 설치된 에이전트 감지
rgt hook pre|post [--agent <name>]         # 에이전트 이벤트 JSON(stdin)에서 자동 수집
```

`--number-format`은 수동 훅 경로에서 숫자 분석에 사용하는 로케일(`.`/`,` 구분자와 부호)을 영구 저장합니다. 유효한 값: `us`(소수점 `.`, 천 단위 `,`), `eu`(소수점 `,`, 천 단위 `.`), 또는 `auto`(파일별로 추론 — 기본값).

`rgt init`은 기존 구성을 절대 파괴하지 않습니다. RGT의 훅 항목을 에이전트 구성에 추가 방식으로 병합합니다 — JSON/JSONC 파일(주석과 후행 쉼표 허용·보존), TOML 구성, 규칙 파일은 다른 모든 훅·키바인딩·플러그인·메모를 바이트 단위로 유지합니다. `--force` 없이 다시 실행하면 아무 변화가 없습니다. `--force`는 RGT 전용 구분 블록만 새로 고칩니다. 구성 파일을 파싱하거나 안전하게 병합할 수 없으면 해당 에이전트가 복구 가능한 오류와 0이 아닌 종료 코드로 보고됩니다 — 다른 에이전트는 계속 구성되고, 기존 파일을 덮어쓰기 전에 `<파일>.rgt.bak` 백업이 보관됩니다.
종료 코드: `0` = 성공, `1` = 하나 이상의 에이전트 실패, `2` = 잘못된 `--agent` 이름.

`rgt init`은 실행 중인 바이너리에서 확인한 **절대 경로**로 CLI를 호출하는 훅 명령을 작성하므로, AI 도구가 실행하는 셸의 PATH에 의존하지 않습니다. 바이너리를 옮긴 경우(예: 패키지 관리자 업그레이드가 새 경로에 설치), `rgt init`을 다시 실행하여 작성된 경로를 갱신하세요.

### Doctor

```bash
rgt doctor   # 훅 설치 및 캡처 상태 진단
```

`rgt doctor`는 `brew doctor` 스타일의 진단입니다. 정상이거나 권고 경고가 있으면 종료 `0`, 명확한 문제(예: 저장소가 초기화되지 않음)가 있으면 `1`입니다. 훅이 설치되었지만 값이 한 번도 기록되지 않은 경우 캡처가 활성화되지 않았을 수 있다고 경고하며, AI 도구가 사용하는 셸 컨텍스트에서 `rgt`이 해석 가능한지 확인하라고 안내합니다(훅 서브프로세스는 페일오픈이라 PATH가 깨지면 아무것도 기록하지 않고 조용히 실패합니다).

### 업데이트
```bash
rgt update [--check] [--yes] [--version <tag>] [--skip-checksum] [--skip-attestation]
```

`rgt update`는 **검증된** 바이너리만 설치합니다. 모든 다운로드는 릴리스의 `checksums.txt`(SHA-256) **및** GitHub Artifact Attestation에 대해 확인됩니다 — 릴리스 자체 자산과 독립적으로 발급된 신뢰 앵커입니다(내장 Sigstore 검증, `gh` 불필요). 두 검증 중 하나라도 없는 릴리스는 0이 아닌 종료로 거부됩니다. `--skip-checksum` / `--skip-attestation`은 명시적이고 안전하지 않음으로 문서화된 옵트아웃이며 기본값이 아닙니다. 버전 비교는 시맨틱 버저닝을 사용합니다. 같거나 이전 태그는 절대 설치되지 않습니다(조용한 다운그레이드 없음). `--version <tag>`는 요청한 태그가 설치된 버전보다 이전이면 경고합니다.

### 출처 추적
```bash
rgt record <file> [--number-format <us|eu|auto>]   # 파일에서 값을 추출하여 추적
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                            # 도출을 검증하고 기록
rgt verify --parents <ids> --operation <op> --result <val>
                                            # 기록하지 않고 도출값 검증
rgt status [--stale-only] [--json]         # 그래프 상태와 최신성
rgt query <node_id> [--json]               # 값의 전체 계보
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths] # 그래프 내보내기
rgt gc [--vacuum]                          # 폐기 노드(오래되고 파생이 없는 값) 제거
rgt mcp                                   # stdio를 통해 Model Context Protocol 서버 실행
```

`rgt record --number-format`은 한 번의 호출에 대해 저장된 형식을 재정의합니다. 그 외에는(`rgt init`의) 저장된 설정을 사용하고 `auto`로 폴백합니다. 숫자 분석은 로케일을 인식합니다. 선행 `-`와 회계 괄호 `(N)`는 값의 일부이고, 활성 형식에 따라 천/소수 구분자가 존중되며, 해당 형식으로 분석할 수 없는 값은 (경고와 함께) 건너뛰고 잘못된 노드로 분할되지 않습니다.

#### Turtle 출처 내보내기

`rgt graph --format ttl`은 PROV-O 엔티티, 활동, 사용 및 코딩 도구 에이전트를 사용하여 결정론적 Turtle을 생성합니다. 프로젝트 범위의 값 IRI는 독립적인 저장소의 내보내기를 RDF 유니온으로 결합할 때 일치하는 로컬 노드 ID를 서로 구별하여 유지합니다. 원래 노드 ID는 `rgt:localNodeId`로 유지됩니다.

기본적으로 소스 문서는 경로가 없는 IRI를 가집니다. 파일이 존재할 경우 안전하게 검증된 프로젝트 상대 경로가 포함됩니다. `--include-absolute-paths` 플래그는 절대 소스 경로를 추가하지만 사용자 이름이나 로컬 디렉터리 이름이 노출될 수 있습니다.

Turtle 내보내기는 10,000개의 기록된 값, 30,000개의 도출 엣지, 64 MiB의 UTF-8, 1초의 명령 기한으로 제한됩니다.

`rgt gc`는 **폐기 노드** — 오래된 것으로 표시되고 아무것도 파생되지 않는 값 — 을 제거하고 제거 개수를 보고합니다. 오래되지 않은 노드나 아직 의존되는 것은 건드리지 않습니다. `rgt gc --vacuum`은 추가로 SQLite `VACUUM`으로 해제된 디스크 페이지를 회수합니다(선택 사항; 기본 실행은 행 삭제만).

### 기간 계산 및 표시

연산: `EXPRESSION`, `DATE_DIFF`, `DURATION_SUM`, `DURATION_AVG`. `DATE_DIFF`는 순서가 있는 두 날짜 사이의 경과 시간을 자동 계산합니다(`date2 - date1`). 음수나 초 미만의 단수는 거부됩니다. 합계와 평균은 중복되지 않는 2~26개의 Duration을 받습니다. 결과는 정확한 정수 초로 저장하며, 초 미만의 단수가 필요한 평균이나 주장은 거부됩니다. 부모 변수는 `parent[0]=a, parent[1]=b, ...`로 매핑됩니다(최대 26개, `a`–`z`).

고정 표시 단위는 `seconds`, `minutes`, `hours`, `days`, `weeks`입니다. 달력의 월과 연도는 길이가 달라 지원하지 않습니다. `--result-unit`은 결과값의 단위를 지정하고 `--unit`은 표시만 바꿉니다. 두 단위 모두 노드 메타데이터로 저장되지 않습니다:

```bash
rgt derive --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days --unit hours
rgt derive --parents <duration1>,<duration2> --operation DURATION_SUM --unit minutes
rgt verify --parents <date1>,<date2> --operation DATE_DIFF --result 45 --result-unit days
rgt query <duration_id> --unit days [--json]
```

쿼리는 기존 `value`를 유지하고 요청한 노드에만 정확한 초를 문자열로 담은 `duration_seconds`와 `selected_unit_display`를 추가합니다. 순환 소수 변환은 근삿값으로 표시하지만 초 값은 정확합니다. 날짜나 숫자를 `--unit`과 함께 조회하면 거부됩니다. `EXPRESSION`은 날짜를 Unix 타임스탬프 초, Duration을 경과 초로 해석하고 Number를 반환하며 경고를 출력합니다.

`--operation`은 대소문자를 구분합니다(`EXPRESSION`/`DATE_DIFF`/`DURATION_SUM`/`DURATION_AVG`); 그 외에는 종료 코드 2로 거부되어 검증이 조용히 건너뛰어지지 않습니다. `--expression`은 4096바이트와 괄호 중첩 256단계로 제한됩니다(초과 시 충돌이 아니라 종료 코드 2로 거부). 표현식은 내장 함수를 비활성화한 채 평가되므로 부모 변수와 `+ - * / % ^`, 괄호만 사용할 수 있어 결정적입니다.

### MCP 서버

`rgt mcp`는 stdio를 통해 RGT를 [Model Context Protocol](https://modelcontextprotocol.io/) 서버로 실행하여 에이전트별 훅 형식 대신 표준화된 MCP 도구로 동일한 작업을 노출합니다 (Claude Code, Cursor, VS Code, ChatGPT 등과 연동 가능):

| 도구 | 기능 |
|------|------|
| `query_provenance` | 값의 전체 출처 계보 추적 |
| `get_status` | 그래프 상태 (전체/활성/오래된 노드) 확인 |
| `record_values` | 파일 내용에서 값을 추출하고 기록 |
| `verify_only` | 기록 없이 도출 연산의 정확성만 검증 |
| `derive_and_verify` | 도출 연산을 검증하고 결과 기록 |

MCP 클라이언트 설정에 프로젝트 디렉터리에서 stdio 하위 프로세스로 실행하도록 구성합니다:

```json
{
  "mcpServers": {
    "rgt": { "command": "rgt", "args": ["mcp"] }
  }
}
```

서버는 실행된 프로젝트 디렉터리에서 작동합니다. 유효하지 않은 입력은 도구 오류로 반환되며, 도구 오류로 인해 서버가 종료되지 않습니다.

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
