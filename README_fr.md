<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Suivi de provenance numerique et date pour les agents de codage IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licence"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#installation">Installer</a> &bull;
  <a href="#demarrage-rapide">Demarrage rapide</a> &bull;
  <a href="#commandes">Commandes</a> &bull;
  <a href="#outils-pris-en-charge">Outils pris en charge</a> &bull;
  <a href="#comment-ca-marche">Comment ca marche</a> &bull;
  <a href="CONTRIBUTING.md">Contribuer</a>
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

RGT offre aux agents de codage IA une memoire persistante et interrogeable de chaque nombre et date qu'ils lisent dans les fichiers sources ou qu'ils derivent par calcul, puis **verifie que chaque derivation est mathematiquement correcte**. Un seul binaire Rust, 13 outils de codage IA pris en charge, des hooks qui enregistrent uniquement les donnees et ne reecrivent jamais les commandes.

## Ce que fait RGT

Les agents raisonnent sur des fichiers qui changent et des calculs qu'ils peuvent se tromper. RGT suit la provenance de chaque valeur numerique et re-verifie les calculs.

| Operation | Ce que fait RGT |
|-----------|-----------------|
| `rgt record <file>` | Extrait chaque nombre et date d'un fichier vers le graphe de provenance |
| `rgt status` | Rapporte les nœuds totaux, actifs et perimes, c'est-a-dire les valeurs encore fiables |
| `rgt derive` | Verifie une valeur calculee par l'agent contre ses parents **avant** de l'enregistrer |
| `rgt query <id>` | Retrace la lignee d'une valeur jusqu'a ses fichiers sources |
| `rgt graph` | Exporte le graphe de dependances (texte, Mermaid ou DOT) |
| `rgt hook` | Capture passive via le mecanisme natif de hook/plugin de chaque agent |

## Pourquoi le suivi de provenance est important

RGT ne mesure pas d'economies : il previent les erreurs silencieuses. Deux modes de defaillance le motivent :

1. **Donnees perimees** : un agent lit un fichier, le fichier change ensuite, et l'agent continue de raisonner avec les anciens nombres. RGT marque les nœuds racines affectes comme **perimes** et propage la peremption a chaque valeur derivee qui en depend (`rgt status`).
2. **Calcul erroné** : un agent calcule `revenue = price * quantity` et se trompe. RGT recalcule l'expression depuis les valeurs parentes de la base et **rejette** les derivations incorrectes (code de sortie 1) avant qu'elles n'entrent dans le graphe.

Les hooks sont **exclusivement de capture de provenance** : ils enregistrent `(path, content)` et ne reecrivent, ne filtrent ni ne bloquent jamais les appels d'outils de l'agent.

## Installation

### Installation rapide (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Installe dans `~/.local/bin`. Ajoutez au PATH si necessaire :
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # ou ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> La formule du tap est actualisee a chaque release GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binaires pre-compiles

Telechargez depuis les [releases](https://github.com/rafael-bianchi/rgt/releases) :
- macOS : `rgt-aarch64-apple-darwin.tar.gz`
- Linux : `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows : `rgt-x86_64-pc-windows-msvc.zip`

> Les binaires macOS et Linux sont publies a chaque release ; les binaires Windows sont produits par le pipeline CI de release et apparaissent des qu'il tourne pour un tag.

### Auto-mise a jour

```bash
rgt update          # remplace atomiquement le binaire par la derniere release
rgt update --check  # verifie une nouvelle version sans l'appliquer
```

### Verifier l'installation

```bash
rgt status   # Affiche l'etat du graphe de provenance (un magasin vierge demarre a 0 nœud)
```

## Demarrage rapide

```bash
# 1. Configurez les hooks de provenance pour votre outil IA
rgt init -g                 # detecte automatiquement chaque agent pris en charge installe
rgt init -g --agent copilot # ou ciblez-en un : copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # les agents a portee projet utilisent les fichiers de regles projet
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo utilisent des fichiers de regles

# 2. Redemarrez votre outil IA, puis :
rgt record budget.csv       # l'agent lit un fichier de donnees et enregistre ses valeurs
rgt status                  # inspectez le graphe
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # l'agent enregistre une derivation verifiee
rgt query node_drv_Z        # tracez la lignee de la valeur derivee
rgt graph --format mermaid  # exportez le graphe de dependances
```

## Comment ca marche

```
  L'agent lit un fichier de donnees                      L'agent calcule une valeur derivee
            |                                                         |
            v                                                         v
  rgt hook post (hook/plugin natif)                       rgt derive --parents <ids>
            |                                                         |
            v                                                 (verification contre les parents)
  rgt record extrait {path, content}                                  |
            |                                                         v
            v                                                   calcul correct ?
  graphe de provenance ── rgt status ── perime ?  ── NON ──> valeur fiable
            |                                   |
            +---------------- OUI --------------->  nœud + dependants marques perimes
```

Trois strategies garantissent la fiabilite du graphe :

1. **Capture** : les hooks/plugins natifs poussent chaque fichier lu par un agent via `rgt record`, si bien que les valeurs entrent dans le graphe sans que l'agent ait a y penser.
2. **Verification** : chaque `rgt derive` est "confiance mais verifie" : RGT recalcule le resultat depuis les valeurs parentes et rejette les mauvais avant insertion.
3. **Peremption** : quand un fichier source change, ses nœuds (et tout ce qui en est derive) sont marques perimes, ce qui permet de dire a l'agent de relire.

## Commandes

### Initialisation et hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configure les hooks, detecte les agents installes
rgt hook pre|post [--agent <name>]         # capture passive depuis le JSON d'evenement de l'agent (stdin)
```

### Provenance
```bash
rgt record <file>                          # extrait et suit les valeurs d'un fichier
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # verifie et enregistre une derivation
rgt verify --parents <ids> --operation <op> --result <val>
                                           # verifie une valeur derivee sans l'enregistrer
rgt status [--stale-only] [--json]         # etat du graphe et peremption
rgt query <node_id> [--json]               # lignee complete d'une valeur
rgt graph [-f text|mermaid|dot]            # exporte le graphe de dependances
```

Operations : `EXPRESSION` (formules comme `a + b * c`) et `DATE_DIFF` (arithmetique de dates, ex. `date2 - date1`). Les variables parentes correspondent a `parent[0]=a, parent[1]=b, ...`.

## Outils pris en charge

RGT configure des hooks de capture de provenance pour 13 outils de codage IA, en utilisant le mecanisme natif de chaque agent :

| Outil | Installation | Methode |
|-------|--------------|---------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | fichier d'instructions (repertoire de config Copilot CLI) |
| **Gemini CLI** | `rgt init -g --agent gemini` | `~/.gemini/hooks.toml` PostToolUse |
| **Mistral Vibe** | `rgt init -g --agent vibe` | hook `pre_tool` (`hooks.toml`) + prompt |
| **OpenCode** | `rgt init -g --agent opencode` | plugin TypeScript |
| **Pi** | `rgt init --agent pi` (ou `-g`) | extension TypeScript |
| **Hermes** | `rgt init --agent hermes` | plugin Python + `plugins.enabled` |
| **Codex CLI** | `rgt init --agent codex` | instructions `AGENTS.md` |
| **Windsurf** | `rgt init --agent windsurf` | `.windsurfrules` |
| **Cline / Roo Code** | `rgt init --agent cline` | `.clinerules` |
| **Google Antigravity** | `rgt init --agent antigravity` | `.agents/rules/antigravity-rgt-rules.md` |
| **Kilo Code** | `rgt init --agent kilocode` | `.kilocode/rules/rgt-rules.md` |

Valeurs `--agent` acceptees : `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, plus les alias `claude`, `roo-code`, `kilo`.

## Donnees et stockage

RGT stocke son graphe dans `.rgt/store.db` (SQLite) a la racine du projet, cree par `rgt init`. Aucun service externe, aucune telemetrie, aucun appel reseau pendant le fonctionnement normal.

## Documentation

- **[AGENTS.md](AGENTS.md)** : instructions que RGT installe pour les agents IA (comment enregistrer et deriver)
- **[CONTRIBUTING.md](CONTRIBUTING.md)** : guide de contribution
- **[CHANGELOG.md](CHANGELOG.md)** : historique des releases

## Remerciements

RGT s'est inspire de [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), un proxy CLI haute performance qui compresse la sortie shell pour les agents de codage IA. RGT suit l'approche de RTK : un binaire unique avec des integrations de hook natives et specifiques par agent, et reprend sa couverture de 13 agents. Là ou RTK filtre la sortie des commandes, RGT suit la provenance numerique et date de ce que les agents lisent et derivent, et verifie que chaque derivation est mathematiquement correcte.

RGT a ete developpe avec l'aide des outils de codage IA [DeepSeek](https://www.deepseek.com/).

## Contribuer

Les contributions sont les bienvenues ! Ouvrez une issue ou une PR sur [GitHub](https://github.com/rafael-bianchi/rgt).

## Licence

Sous [Licence Apache, version 2.0](LICENSE).
