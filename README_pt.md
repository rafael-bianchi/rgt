<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Rastreamento da procedência de números e datas para agentes de codificação com IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licença"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#instalação">Instalação</a> &bull;
  <a href="#início-rápido">Início rápido</a> &bull;
  <a href="#comandos">Comandos</a> &bull;
  <a href="#ferramentas-de-ia-compatíveis">Ferramentas de IA compatíveis</a> &bull;
  <a href="#como-funciona">Como funciona</a> &bull;
  <a href="CONTRIBUTING.md">Contribuir</a>
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

O RGT dá aos agentes de codificação com IA uma memória persistente e consultável de cada número e data que leem de arquivos fonte ou derivam por cálculos e, em seguida, **verifica se cada derivação está matematicamente correta**. Um único binário em Rust, 13 ferramentas de codificação com IA compatíveis, hooks que apenas registram dados e nunca reescrevem comandos.

## O que o RGT faz

Agentes raciocinam sobre arquivos que mudam e sobre cálculos em que podem errar. O RGT rastreia a procedência de cada valor numérico e verifica novamente as contas.

| Operação | O que o RGT faz |
|----------|-----------------|
| `rgt record <file>` | Extrai cada número e data de um arquivo para o grafo de procedência |
| `rgt status` | Informa o total de nós, os nós ativos e obsoletos e indica quais valores ainda são confiáveis |
| `rgt derive` | Verifica um valor calculado pelo agente com base em seus nós pais **antes** de registrá-lo |
| `rgt query <id>` | Rastreia a linhagem de um valor até os arquivos de origem |
| `rgt graph` | Exporta o DAG de dependências (texto, Mermaid ou DOT) |
| `rgt hook` | Captura passiva por meio do mecanismo nativo de hook/plugin de cada agente |

## Por que o rastreamento de procedência importa

O RGT não mede ganhos: ele evita erros silenciosos. Dois modos de falha justificam seu uso:

1. **Dados obsoletos**: um agente lê um arquivo, depois o arquivo muda e o agente continua raciocinando com os números antigos. O RGT marca os nós raiz afetados como **obsoletos** e propaga a obsolescência para cada valor derivado que deles depende (`rgt status`).
2. **Conta errada**: um agente calcula `revenue = price * quantity` e erra. O RGT recalcula a expressão a partir dos valores pai no banco de dados e **rejeita** as derivações incorretas (código de saída 1) antes que entrem no grafo.

Os hooks são **exclusivamente de captura de procedência**: registram `(path, content)` e nunca reescrevem, filtram nem bloqueiam as chamadas de ferramenta do agente.

## Instalação

### Instalação rápida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Instala em `~/.local/bin`. Adicione-o ao PATH se necessário:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # ou ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> A fórmula do tap é atualizada a cada versão publicada no GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binários pré-compilados

Baixe em [releases](https://github.com/rafael-bianchi/rgt/releases):
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> Binários pré-compilados para macOS, Linux e Windows são publicados a cada release.

### Atualização automática

```bash
rgt update          # substitui atomicamente o binário pela release mais recente
rgt update --check  # verifica se há uma versão nova sem aplicá-la
```

### Verificar a instalação

```bash
rgt status   # Mostra o estado do grafo de procedência (um armazenamento novo começa em 0 nós)
```

## Início rápido

```bash
# 1. Configure os hooks de procedência para sua ferramenta de IA
rgt init -g                 # detecta automaticamente cada agente compatível instalado
rgt init -g --agent copilot # ou especifique um: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # agentes no escopo do projeto usam arquivos de regras do projeto
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo usam arquivos de regras

# 2. Reinicie sua ferramenta de IA e então:
rgt record budget.csv       # o agente lê um arquivo de dados e registra seus valores
rgt status                  # inspecione o grafo
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # o agente registra uma derivação verificada
rgt query node_drv_Z        # rastreie a linhagem do valor derivado
rgt graph --format mermaid  # exporte o grafo de dependências
rgt graph --format ttl      # exporta Turtle PROV-O
```

## Como funciona

```
  O agente lê um arquivo de dados                      O agente calcula um valor derivado
            |                                                    |
            v                                                    v
  rgt hook post (hook/plugin nativo)                    rgt derive --parents <ids>
            |                                                    |
            v                                  (verificação com base nos nós pais)
  rgt record extrai {path, content}                             |
            |                                                    v
            v                                               conta correta?
  grafo de procedência ── rgt status ── obsoleto? ── NÃO ──> valor confiável
            |                                        |
            +---------------- SIM -------------------->  nó + nós dependentes marcados obsoletos
```

Três estratégias mantêm o grafo confiável:

1. **Captura**: hooks/plugins nativos encaminham cada arquivo que um agente lê por meio do `rgt record`; assim, os valores entram no grafo sem o agente precisar se lembrar de chamá-lo.
2. **Verificação**: cada `rgt derive` aplica o princípio "confie, mas verifique": o RGT recalcula o resultado a partir dos valores pai e rejeita os errados antes da inserção.
3. **Obsolescência**: quando um arquivo de origem muda, seus nós (e tudo o que deles deriva) são marcados como obsoletos, permitindo orientar o agente a reler o arquivo.

## Comandos

### Inicialização e hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configura hooks, detecta agentes instalados
rgt hook pre|post [--agent <name>]         # captura passiva do JSON de evento do agente (stdin)
```

### Procedência
```bash
rgt record <file>                          # extrai e rastreia valores de um arquivo
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verifica e registra uma derivação
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verifica um valor derivado sem registrar
rgt status [--stale-only] [--json]         # estado do grafo e obsolescência
rgt query <node_id> [--json]               # linhagem completa de um valor
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths] # exporta o grafo
```

Operações: `EXPRESSION` (fórmulas como `a + b * c`) e `DATE_DIFF` (aritmética de datas, ex. `date2 - date1`). As variáveis pai correspondem a `parent[0]=a, parent[1]=b, ...`.

## Ferramentas de IA compatíveis

O RGT configura hooks de captura de procedência para 13 ferramentas de codificação com IA, usando o mecanismo nativo de cada agente:

| Ferramenta | Instalação | Método |
|------------|------------|--------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks do Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | arquivo de instruções (diretório de configuração do Copilot CLI) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | hook `pre_tool` (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | plugin TypeScript |
| **Pi** | `rgt init --agent pi` (ou `-g`) | extensão TypeScript |
| **Hermes** | `rgt init --agent hermes` | plugin Python + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | instruções `AGENTS.md` |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Valores `--agent` aceitos: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, além dos aliases `claude`, `roo-code`, `kilo`.

## Dados e armazenamento

O RGT armazena seu grafo em `.rgt/store.db` (SQLite), na raiz do projeto; `rgt init` cria esse arquivo. Sem serviços externos, sem telemetria e sem chamadas de rede durante a operação normal.

## Documentação

- **[AGENTS.md](AGENTS.md)**: instruções que o RGT instala para agentes de IA (como registrar e derivar)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: guia de contribuição
- **[CHANGELOG.md](CHANGELOG.md)**: histórico de releases

## Agradecimentos

O RGT foi inspirado no [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), um proxy CLI de alto desempenho que compacta a saída do shell para agentes de codificação com IA. O RGT segue a abordagem do RTK: um único binário com integrações de hook nativas e específicas para cada agente, e espelha sua cobertura de 13 agentes. Enquanto o RTK filtra a saída de comandos, o RGT rastreia a procedência dos números e das datas que os agentes leem e derivam e verifica se cada derivação está matematicamente correta.

O RGT foi desenvolvido com a ajuda das ferramentas de codificação com IA da [DeepSeek](https://www.deepseek.com/).

## Contribuir

Contribuições são bem-vindas! Abra uma issue ou PR no [GitHub](https://github.com/rafael-bianchi/rgt).

## Licença

Licenciado sob a [Licença Apache, versão 2.0](LICENSE).
