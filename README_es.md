<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Seguimiento de la procedencia de números y fechas para agentes de codificación con IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licencia"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#instalación">Instalación</a> &bull;
  <a href="#inicio-rápido">Inicio rápido</a> &bull;
  <a href="#comandos">Comandos</a> &bull;
  <a href="#herramientas-de-ia-compatibles">Herramientas de IA compatibles</a> &bull;
  <a href="#cómo-funciona">Cómo funciona</a> &bull;
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

RGT da a los agentes de codificación con IA una memoria persistente y consultable de cada número y fecha que leen de los archivos fuente o derivan mediante cálculos, y luego **verifica que cada derivación sea matemáticamente correcta**. Un único binario de Rust, 13 herramientas de codificación con IA compatibles, hooks que solo registran datos y nunca reescriben comandos.

## Qué hace RGT

Los agentes razonan a partir de archivos que cambian y realizan cálculos en los que pueden equivocarse. RGT rastrea la procedencia de cada valor numérico y vuelve a comprobar las operaciones.

| Operación | Qué hace RGT |
|-----------|--------------|
| `rgt record <file>` | Extrae cada número y fecha de un archivo y los incorpora al grafo de procedencia |
| `rgt status` | Informa del número total de nodos, de los nodos activos y obsoletos, e indica qué valores siguen siendo fiables |
| `rgt derive` | Verifica un valor calculado por el agente a partir de sus nodos padre **antes** de registrarlo |
| `rgt query <id>` | Rastrea el linaje de un valor hasta sus archivos fuente |
| `rgt graph` | Exporta el DAG de dependencias (texto, Mermaid o DOT) |
| `rgt hook` | Captura pasiva mediante el mecanismo nativo de hook/plugin de cada agente |

## Por qué importa el seguimiento de procedencia

RGT no mide ahorros: previene errores silenciosos. Su uso responde a dos modos de fallo:

1. **Datos obsoletos**: un agente lee un archivo, luego el archivo cambia y el agente sigue razonando con los números antiguos. RGT marca los nodos raíz afectados como **obsoletos** y propaga la obsolescencia a cada valor derivado que dependa de ellos (`rgt status`).
2. **Operación errónea**: un agente calcula `revenue = price * quantity` y se equivoca. RGT recalcula la expresión a partir de los valores padre de la base de datos y **rechaza** las derivaciones incorrectas (código de salida 1) antes de que entren en el grafo.

Los hooks son **exclusivamente de captura de procedencia**: registran `(path, content)` y nunca reescriben, filtran ni bloquean las llamadas a herramientas del agente.

## Instalación

### Instalación rápida (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Se instala en `~/.local/bin`. Agrégalo al PATH si es necesario:
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # o ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> La fórmula del tap se actualiza con cada versión publicada en GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binarios precompilados

Descarga desde [releases](https://github.com/rafael-bianchi/rgt/releases):
- macOS: `rgt-aarch64-apple-darwin.tar.gz`
- Linux: `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows: `rgt-x86_64-pc-windows-msvc.zip`

> Los binarios precompilados para macOS, Linux y Windows se publican con cada release.

### Actualización automática

```bash
rgt update          # reemplaza atómicamente el binario con la última release
rgt update --check  # comprueba si hay una versión nueva sin aplicarla
```

### Verificar la instalación

```bash
rgt status   # Muestra el estado del grafo de procedencia (un almacén nuevo empieza en 0 nodos)
```

## Inicio rápido

```bash
# 1. Configura hooks de procedencia para tu herramienta de IA
rgt init -g                 # detecta automáticamente cada agente compatible instalado
rgt init -g --agent copilot # o especifica uno: copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # los agentes de ámbito de proyecto usan archivos de reglas del proyecto
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo usan archivos de reglas

# 2. Reinicia tu herramienta de IA y luego:
rgt record budget.csv       # el agente lee un archivo de datos y registra sus valores
rgt status                  # inspecciona el grafo
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # el agente registra una derivación verificada
rgt query node_drv_Z        # rastrea el linaje del valor derivado
rgt graph --format mermaid  # exporta el grafo de dependencias
```

## Cómo funciona

```
  El agente lee un archivo de datos                    El agente calcula un valor derivado
            |                                                     |
            v                                                     v
  rgt hook post (hook/plugin nativo)                   rgt derive --parents <ids>
            |                                                     |
            v                                  (verificación a partir de los valores padre)
  rgt record extrae {path, content}                              |
            |                                                     v
            v                                               ¿cálculo correcto?
  grafo de procedencia ── rgt status ── obsoleto? ── NO ──> valor fiable
            |                                        |
            +---------------- SÍ -------------------->  nodo + dependientes marcados obsoletos
```

Tres estrategias mantienen fiable el grafo:

1. **Captura**: los hooks/plugins nativos envían cada archivo que el agente lee a través de `rgt record`, de modo que los valores entran al grafo sin que el agente tenga que acordarse de llamarlo.
2. **Verificación**: cada `rgt derive` aplica el principio «confía, pero verifica»: RGT recalcula el resultado a partir de los valores padre y rechaza los incorrectos antes de insertarlos.
3. **Obsolescencia**: cuando un archivo fuente cambia, sus nodos (y todo lo derivado de ellos) se marcan como obsoletos, de modo que se puede indicar al agente que relea.

## Comandos

### Inicialización y hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configura hooks, detecta agentes instalados
rgt hook pre|post [--agent <name>]         # captura pasiva desde el JSON de evento del agente (stdin)
```

### Procedencia
```bash
rgt record <file>                          # extrae y rastrea valores de un archivo
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verifica y registra una derivación
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verifica un valor derivado sin registrarlo
rgt status [--stale-only] [--json]         # estado del grafo y obsolescencia
rgt query <node_id> [--json]               # linaje completo de un valor
rgt graph [-f text|mermaid|dot]            # exporta el DAG de dependencias
```

Operaciones: `EXPRESSION` (fórmulas como `a + b * c`) y `DATE_DIFF` (aritmética de fechas, p. ej. `date2 - date1`). Las variables padre se asignan como `parent[0]=a, parent[1]=b, ...`.

## Herramientas de IA compatibles

RGT configura hooks de captura de procedencia para 13 herramientas de IA, usando el mecanismo nativo de cada agente:

| Herramienta | Instalación | Método |
|-------------|-------------|--------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks de Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | archivo de instrucciones (directorio de configuración de Copilot CLI) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | hook `pre_tool` (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | plugin TypeScript |
| **Pi** | `rgt init --agent pi` (o `-g`) | extensión TypeScript |
| **Hermes** | `rgt init --agent hermes` | plugin Python + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | instrucciones `AGENTS.md` |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Valores `--agent` aceptados: `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, además de los alias `claude`, `roo-code`, `kilo`.

## Datos y almacenamiento

RGT guarda su grafo en `.rgt/store.db` (SQLite), en la raíz del proyecto; `rgt init` crea este archivo. Sin servicios externos, sin telemetría y sin llamadas de red durante el funcionamiento normal.

## Documentación

- **[AGENTS.md](AGENTS.md)**: instrucciones que RGT instala para los agentes de IA (cómo registrar y derivar)
- **[CONTRIBUTING.md](CONTRIBUTING.md)**: guía de contribución
- **[CHANGELOG.md](CHANGELOG.md)**: historial de releases

## Agradecimientos

RGT se inspiró en [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), un proxy CLI de alto rendimiento que comprime la salida del shell para agentes de codificación con IA. RGT sigue el enfoque de RTK: un único binario con integraciones de hook nativas y específicas para cada agente, y replica su cobertura de 13 agentes. Mientras que RTK filtra la salida de comandos, RGT rastrea la procedencia de los números y las fechas que los agentes leen y derivan, y verifica que cada derivación sea matemáticamente correcta.

RGT fue desarrollado con la ayuda de las herramientas de codificación con IA de [DeepSeek](https://www.deepseek.com/).

## Contribuir

¡Las contribuciones son bienvenidas! Abre una issue o PR en [GitHub](https://github.com/rafael-bianchi/rgt).

## Licencia

Distribuido bajo la [Licencia Apache, versión 2.0](LICENSE).
