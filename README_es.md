<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Segun miento de procedencia numerica y de fechas para agentes de codificacion con IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licencia"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#instalacion">Instalacion</a> &bull;
  <a href="#inicio-rapido">Inicio rapido</a> &bull;
  <a href="#comandos">Comandos</a> &bull;
  <a href="#herramientas-de-ia-compatibles">Herramientas de IA compatibles</a> &bull;
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

RGT da a los agentes de codificacion con IA una memoria persistente y consultable de cada numero y fecha que leen de los archivos fuente o derivan mediante calculos, y luego **verifica que cada derivacion sea matematicamente correcta**. Un unico binario de Rust, 13 herramientas de codificacion con IA compatibles, hooks que solo registran datos y nunca reescriben comandos.

## Que hace RGT

Los agentes razonan sobre archivos que cambian y sobre aritmetica que pueden errar. RGT rastrea la procedencia de cada valor numerico y vuelve a comprobar las operaciones.

| Operacion | Que hace RGT |
|-----------|--------------|
| `rgt record <file>` | Extrae cada numero y fecha de un archivo hacia el grafo de procedencia |
| `rgt status` | Reporta nodos totales, activos y obsoletos, es decir, que valores siguen siendo fiables |
| `rgt derive` | Verifica un valor calculado por el agente contra sus padres **antes** de registrarlo |
| `rgt query <id>` | Rastrea el linaje de un valor hasta sus archivos fuente |
| `rgt graph` | Exporta el DAG de dependencias (texto, Mermaid o DOT) |
| `rgt hook` | Captura pasiva mediante el mecanismo nativo de hook/plugin de cada agente |

## Por que importa el seguimiento de procedencia

RGT no mide ahorros: previene errores silenciosos. Dos modos de fallo lo motivan:

1. **Datos obsoletos**: un agente lee un archivo, luego el archivo cambia y el agente sigue razonando con los numeros antiguos. RGT marca los nodos raiz afectados como **obsoletos** y propaga la obsolescencia a cada valor derivado que dependa de ellos (`rgt status`).
2. **Operacion erronea**: un agente calcula `revenue = price * quantity` y se equivoca. RGT recalcula la expresion a partir de los valores padre de la base de datos y **rechaza** las derivaciones incorrectas (codigo de salida 1) antes de que entren al grafo.

Los hooks son **exclusivamente de captura de procedencia**: registran `(path, content)` y nunca reescriben, filtran ni bloquean las llamadas de herramienta del agente.

## Instalacion

### Instalacion rapida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Instala en `~/.local/bin`. Agregalo al PATH si es necesario:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # o ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> La formula del tap se actualiza con cada release de GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binarios precompilados

Descarga desde [releases](https://github.com/rafael-bianchi/rgt/releases):
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> Los binarios de macOS y Linux se publican con cada release; los de Windows los produce el pipeline CI de releases y aparecen cuando este se ejecuta para un tag.

### Autoadministrado

```bash
rgt update          # reemplaza atomicamente el binario con la ultima release
rgt update --check  # comprueba si hay una version nueva sin aplicarla
```

### Verificar instalacion

```bash
rgt status   # Muestra el estado del grafo de procedencia (un almacen nuevo empieza en 0 nodos)
```

## Inicio rapido

```bash
# 1. Configura hooks de procedencia para tu herramienta de IA
rgt init -g                 # detecta automaticamente cada agente compatible instalado
rgt init -g --agent copilot # o apunta a uno: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # los agentes de ambito proyecto usan archivos de reglas del proyecto
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo usan archivos de reglas

# 2. Reinicia tu herramienta de IA y luego:
rgt record budget.csv       # el agente lee un archivo de datos y registra sus valores
rgt status                  # inspecciona el grafo
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # el agente registra una derivacion verificada
rgt query node_drv_Z        # rastrea el linaje del valor derivado
rgt graph --format mermaid  # exporta el grafo de dependencias
```

## Como funciona

```
  El agente lee un archivo de datos                    El agente calcula un valor derivado
            |                                                     |
            v                                                     v
  rgt hook post (hook/plugin nativo)                   rgt derive --parents <ids>
            |                                                     |
            v                                           (verificacion contra los padres)
  rgt record extrae {path, content}                              |
            |                                                     v
            v                                               calculo correcto?
  grafo de procedencia ── rgt status ── obsoleto? ── NO ──> valor fiable
            |                                        |
            +---------------- SI -------------------->  nodo + dependientes marcados obsoletos
```

Tres estrategias mantienen fiable el grafo:

1. **Captura**: los hooks/plugins nativos empujan cada archivo que el agente lee a traves de `rgt record`, de modo que los valores entran al grafo sin que el agente tenga que acordarse de llamarlo.
2. **Verificacion**: cada `rgt derive` es "confianza pero verificacion": RGT recalcula el resultado a partir de los valores padre y rechaza los incorrectos antes de insertarlos.
3. **Obsolescencia**: cuando un archivo fuente cambia, sus nodos (y todo lo derivado de ellos) se marcan como obsoletos, de modo que se puede indicar al agente que relea.

## Comandos

### Inicializacion y hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configura hooks, detecta agentes instalados
rgt hook pre|post [--agent <name>]         # captura pasiva desde el JSON de evento del agente (stdin)
```

### Procedencia
```bash
rgt record <file>                          # extrae y rastrea valores de un archivo
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verifica y registra una derivacion
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verifica un valor derivado sin registrarlo
rgt status [--stale-only] [--json]         # estado del grafo y obsolescencia
rgt query <node_id> [--json]               # linaje completo de un valor
rgt graph [-f text|mermaid|dot]            # exporta el DAG de dependencias
```

Operaciones: `EXPRESSION` (formulas como `a + b * c`) y `DATE_DIFF` (aritmetica de fechas, p. ej. `date2 - date1`). Las variables padre se asignan como `parent[0]=a, parent[1]=b, ...`.

## Herramientas de IA compatibles

RGT configura hooks de captura de procedencia para 13 herramientas de IA, usando el mecanismo nativo de cada agente:

| Herramienta | Instalacion | Metodo |
|-------------|-------------|--------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks de Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | archivo de instrucciones (directorio de config de Copilot CLI) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | hook `pre_tool` (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | plugin TypeScript |
| **Pi** | `rgt init --agent pi` (o `-g`) | extension TypeScript |
| **Hermes** | `rgt init --agent hermes` | plugin Python + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | instrucciones `AGENTS.md` |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Valores `--agent` aceptados: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, mas los alias `claude`, `roo-code`, `kilo`.

## Datos y almacenamiento

RGT guarda su grafo en `.rgt/store.db` (SQLite) en la raiz del proyecto, creado por `rgt init`. Sin servicios externos, sin telemetria, sin llamadas de red durante el funcionamiento normal.

## Documentacion

- **[AGENTS.md](AGENTS.md)**: instrucciones que RGT instala para los agentes de IA (como registrar y derivar)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: guia de contribucion
- **[CHANGELOG.md](CHANGELOG.md)**: historial de releases

## Agradecimientos

RGT se inspiro en [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), un proxy CLI de alto rendimiento que comprime la salida de shell para agentes de codificacion con IA. RGT sigue el enfoque de RTK: un unico binario con integraciones de hook nativas y especificas por agente, y replica su cobertura de 13 agentes. Donde RTK filtra la salida de comandos, RGT rastrea la procedencia numerica y de fechas de lo que los agentes leen y derivan, y verifica que cada derivacion sea matematicamente correcta.

RGT fue desarrollado con la ayuda de las herramientas de codificacion con IA [DeepSeek](https://www.deepseek.com/).

## Contribuir

¡Las contribuciones son bienvenidas! Abre una issue o PR en [GitHub](https://github.com/rafael-bianchi/rgt).

## Licencia

Licenciado bajo la [Apache License, Version 2.0](LICENSE).
