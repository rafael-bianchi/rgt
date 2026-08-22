<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Rastreamento de procedencia numerica e de datas para agentes de codificacao com IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licenca"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#instalacao">Instalacao</a> &bull;
  <a href="#inicio-rapido">Inicio rapido</a> &bull;
  <a href="#comandos">Comandos</a> &bull;
  <a href="#ferramentas-de-ia-compatíveis">Ferramentas de IA compativeis</a> &bull;
  <a href="#como-funciona">Como funciona</a> &bull;
  <a href="CONTRIBUTING.md">Contribuir</a>
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

RGT da aos agentes de codificacao com IA uma memoria persistente e consultavel de cada numero e data que leem de arquivos fonte ou derivam por calculos e, em seguida, **verifica se cada derivacao esta matematicamente correta**. Um unico binario em Rust, 13 ferramentas de codificacao com IA compatíveis, hooks que apenas registram dados e nunca reescrevem comandos.

## O que o RGT faz

Agentes raciocinam sobre arquivos que mudam e sobre aritmetica em que podem errar. O RGT rastreia a procedencia de cada valor numerico e recheca as contas.

| Operacao | O que o RGT faz |
|----------|-----------------|
| `rgt record <file>` | Extrai cada numero e data de um arquivo para o grafo de procedencia |
| `rgt status` | Reporta nos totais, ativos e obsoletos, ou seja, quais valores ainda sao confiaveis |
| `rgt derive` | Verifica um valor calculado pelo agente contra seus pais **antes** de registra-lo |
| `rgt query <id>` | Rastreia a linhagem de um valor de volta aos arquivos fonte |
| `rgt graph` | Exporta o DAG de dependencias (texto, Mermaid ou DOT) |
| `rgt hook` | Captura passiva via mecanismo nativo de hook/plugin de cada agente |

## Por que o rastreamento de procedencia importa

O RGT nao mede economias: ele previne erros silenciosos. Dois modos de falha o motivam:

1. **Dados obsoletos**: um agente le um arquivo, depois o arquivo muda e o agente continua raciocinando com os numeros antigos. O RGT marca os nos raiz afetados como **obsoletos** e propaga a obsolescencia para cada valor derivado que deles depende (`rgt status`).
2. **Conta errada**: um agente calcula `revenue = price * quantity` e erra. O RGT recalcula a expressao a partir dos valores pai no banco e **rejeita** as derivacoes incorretas (codigo de saida 1) antes que entrem no grafo.

Os hooks sao **exclusivamente de captura de procedencia**: registram `(path, content)` e nunca reescrevem, filtram nem bloqueiam as chamadas de ferramenta do agente.

## Instalacao

### Instalacao rapida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Instala em `~/.local/bin`. Adicione ao PATH se necessario:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # ou ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> A formula do tap e atualizada a cada release do GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binarios pre-compilados

Baixe em [releases](https://github.com/rafael-bianchi/rgt/releases):
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> Os binarios de macOS e Linux sao publicados a cada release; os de Windows sao produzidos pelo pipeline de release do CI e aparecem quando ele executa para uma tag.

### Auto-atualizacao

```bash
rgt update          # substitui atomicamente o binario pela release mais recente
rgt update --check  # verifica se ha uma versao nova sem aplicar
```

### Verificar instalacao

```bash
rgt status   # Mostra o estado do grafo de procedencia (um armazenamento novo comeca em 0 nos)
```

## Inicio rapido

```bash
# 1. Configure os hooks de procedencia para sua ferramenta de IA
rgt init -g                 # detecta automaticamente cada agente compativel instalado
rgt init -g --agent copilot # ou aponte para um: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # agentes de escopo de projeto usam arquivos de regras do projeto
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo usam arquivos de regras

# 2. Reinicie sua ferramenta de IA e entao:
rgt record budget.csv       # o agente le um arquivo de dados e registra seus valores
rgt status                  # inspecione o grafo
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # o agente registra uma derivacao verificada
rgt query node_drv_Z        # rastreie a linhagem do valor derivado
rgt graph --format mermaid  # exporte o grafo de dependencias
```

## Como funciona

```
  O agente le um arquivo de dados                      O agente calcula um valor derivado
            |                                                    |
            v                                                    v
  rgt hook post (hook/plugin nativo)                    rgt derive --parents <ids>
            |                                                    |
            v                                         (verificacao contra os pais)
  rgt record extrai {path, content}                             |
            |                                                    v
            v                                               conta correta?
  grafo de procedencia ── rgt status ── obsoleto? ── NAO ──> valor confiavel
            |                                        |
            +---------------- SIM -------------------->  no + dependentes marcados obsoletos
```

Tres estrategias mantem o grafo confiavel:

1. **Captura**: hooks/plugins nativos empurram cada arquivo que um agente le por `rgt record`, entao os valores entram no grafo sem o agente precisar lembrar de chama-lo.
2. **Verificacao**: cada `rgt derive` e "confie, mas verifique": o RGT recalcula o resultado a partir dos valores pai e rejeita os errados antes da insercao.
3. **Obsolescencia**: quando um arquivo fonte muda, seus nos (e tudo o que deles deriva) sao marcados como obsoletos, permitindo avisar o agente para reler.

## Comandos

### Inicializacao e hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configura hooks, detecta agentes instalados
rgt hook pre|post [--agent <name>]         # captura passiva do JSON de evento do agente (stdin)
```

### Procedencia
```bash
rgt record <file>                          # extrai e rastreia valores de um arquivo
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verifica e registra uma derivacao
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verifica um valor derivado sem registrar
rgt status [--stale-only] [--json]         # estado do grafo e obsolescencia
rgt query <node_id> [--json]               # linhagem completa de um valor
rgt graph [-f text|mermaid|dot]            # exporta o DAG de dependencias
```

Operacoes: `EXPRESSION` (formulas como `a + b * c`) e `DATE_DIFF` (aritmetica de datas, ex. `date2 - date1`). As variaveis pai mapeiam como `parent[0]=a, parent[1]=b, ...`.

## Ferramentas de IA compativeis

O RGT configura hooks de captura de procedencia para 13 ferramentas de IA, usando o mecanismo nativo de cada agente:

| Ferramenta | Instalacao | Metodo |
|------------|------------|--------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks do Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | arquivo de instrucoes (diretorio de config do Copilot CLI) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | hook `pre_tool` (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | plugin TypeScript |
| **Pi** | `rgt init --agent pi` (ou `-g`) | extensao TypeScript |
| **Hermes** | `rgt init --agent hermes` | plugin Python + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | instrucoes `AGENTS.md` |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Valores `--agent` aceitos: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, mais os aliases `claude`, `roo-code`, `kilo`.

## Dados e armazenamento

O RGT armazena seu grafo em `.rgt/store.db` (SQLite) na raiz do projeto, criado por `rgt init`. Sem servicos externos, sem telemetria, sem chamadas de rede durante a operacao normal.

## Documentacao

- **[AGENTS.md](AGENTS.md)**: instrucoes que o RGT instala para agentes de IA (como registrar e derivar)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: guia de contribuicao
- **[CHANGELOG.md](CHANGELOG.md)**: historico de releases

## Agradecimentos

O RGT foi inspirado no [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), um proxy CLI de alto desempenho que compacta a saida do shell para agentes de codificacao com IA. O RGT segue a abordagem do RTK: um unico binario com integracoes de hook nativas e especificas por agente, e espelha sua cobertura de 13 agentes. Onde o RTK filtra a saida de comandos, o RGT rastreia a procedencia numerica e de datas do que os agentes leem e derivam e verifica se cada derivacao esta matematicamente correta.

O RGT foi desenvolvido com a ajuda das ferramentas de codificacao com IA [DeepSeek](https://www.deepseek.com/).

## Contribuir

Contribuicoes sao bem-vindas! Abra uma issue ou PR no [GitHub](https://github.com/rafael-bianchi/rgt).

## Licenca

Licenciado sob a [Apache License, Version 2.0](LICENSE).
