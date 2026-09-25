<p align="center">
  <img src="docs/rgt_crab.jpeg" alt="RGT - Rust Graph Tracker" width="200">
</p>

<p align="center">
  <strong>RGT - Rust Graph Tracker</strong>
</p>

<p align="center">
  <strong>Suivi de la provenance des nombres et des dates pour les agents de codage IA</strong>
</p>

<p align="center">
  <a href="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml"><img src="https://github.com/rafael-bianchi/rgt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/rafael-bianchi/rgt/releases"><img src="https://img.shields.io/github/v/release/rafael-bianchi/rgt" alt="Release"></a>
  <a href="#licence"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache-2.0"></a>
</p>

<p align="center">
  <a href="#installation">Installer</a> &bull;
  <a href="#démarrage-rapide">Démarrage rapide</a> &bull;
  <a href="#commandes">Commandes</a> &bull;
  <a href="#outils-pris-en-charge">Outils pris en charge</a> &bull;
  <a href="#comment-ça-marche">Comment ça marche</a> &bull;
  <a href="CONTRIBUTING.md">Contribuer</a>
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

RGT offre aux agents de codage IA une mémoire persistante et interrogeable de chaque nombre et date qu'ils lisent dans les fichiers sources ou qu'ils dérivent par calcul, puis **vérifie que chaque dérivation est mathématiquement correcte**. Un seul binaire Rust, 13 outils de codage IA pris en charge, des hooks qui enregistrent uniquement les données et ne réécrivent jamais les commandes.

## Ce que fait RGT

Les agents raisonnent à partir de fichiers qui changent et effectuent des calculs dans lesquels ils peuvent se tromper. RGT suit la provenance de chaque valeur numérique et revérifie les calculs.

| Opération | Ce que fait RGT |
|-----------|-----------------|
| `rgt record <file>` | Extrait chaque nombre et date d'un fichier et les ajoute au graphe de provenance |
| `rgt status` | Indique le nombre total de nœuds, ainsi que les nœuds actifs et obsolètes, et montre quelles valeurs restent fiables |
| `rgt derive` | Vérifie une valeur calculée par l'agent à partir de ses nœuds parents **avant** de l'enregistrer |
| `rgt query <id>` | Retrace la lignée d'une valeur jusqu'à ses fichiers sources |
| `rgt graph` | Exporte le graphe de dépendances (texte, Mermaid ou DOT) |
| `rgt hook` | Capture passive via le mécanisme natif de hook/plugin de chaque agent |

## Pourquoi le suivi de la provenance est important

RGT ne mesure pas les économies : il prévient les erreurs silencieuses. Deux modes de défaillance justifient son existence :

1. **Données obsolètes** : un agent lit un fichier, le fichier change ensuite, et l'agent continue de raisonner avec les anciens nombres. RGT marque les nœuds racines affectés comme **obsolètes** et propage l'obsolescence à chaque valeur dérivée qui en dépend (`rgt status`).
2. **Calcul erroné** : un agent calcule `revenue = price * quantity` et se trompe. RGT recalcule l'expression à partir des valeurs parentes de la base de données et **rejette** les dérivations incorrectes (code de sortie 1) avant qu'elles n'entrent dans le graphe.

Les hooks servent **exclusivement à capturer la provenance** : ils enregistrent `(path, content)` et ne réécrivent, ne filtrent ni ne bloquent jamais les appels d'outils de l'agent.

## Installation

### Installation rapide (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/rafael-bianchi/rgt/main/install.sh | sh
```

> Installe dans `~/.local/bin`. Ajoutez-le au PATH si nécessaire :
> ```bash
> echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc  # ou ~/.bashrc
> ```

### Homebrew (tap)

```bash
brew install rafael-bianchi/rgt/rgt
```

> La formule du tap est mise à jour à chaque version publiée sur GitHub.

### Cargo

```bash
cargo install --git https://github.com/rafael-bianchi/rgt
```

### Binaires précompilés

Téléchargez-les depuis les [releases](https://github.com/rafael-bianchi/rgt/releases) :
- macOS : `rgt-aarch64-apple-darwin.tar.gz`
- Linux : `rgt-x86_64-unknown-linux-musl.tar.gz` / `rgt-aarch64-unknown-linux-gnu.tar.gz`
- Windows : `rgt-x86_64-pc-windows-msvc.zip`

> Des binaires pré-compilés pour macOS, Linux et Windows sont publiés à chaque release.

### Mise à jour automatique

```bash
rgt update          # remplace atomiquement le binaire par la dernière release
rgt update --check  # vérifie si une nouvelle version existe sans l'appliquer
```

### Vérifier l'installation

```bash
rgt status   # Affiche l'état du graphe de provenance (un magasin vierge démarre à 0 nœud)
```

## Démarrage rapide

```bash
# 1. Configurez les hooks de provenance pour votre outil IA
rgt init -g                 # détecte automatiquement chaque agent pris en charge installé
rgt init -g --agent copilot # ou ciblez-en un : copilot, gemini, vibe, opencode, pi, hermes, ...
rgt init --agent cline      # les agents à l'échelle du projet utilisent les fichiers de règles du projet
rgt init --agent codex      # Codex / Windsurf / Cline / Antigravity / Kilo utilisent des fichiers de règles

# 2. Redémarrez votre outil IA, puis :
rgt record budget.csv       # l'agent lit un fichier de données et enregistre ses valeurs
rgt status                  # inspectez le graphe
rgt derive --parents node_raw_X,node_raw_Y --operation EXPRESSION --expression "a - b" --result 60000
                            # l'agent enregistre une dérivation vérifiée
rgt query node_drv_Z        # tracez la lignée de la valeur dérivée
rgt graph --format mermaid  # exportez le graphe de dépendances
rgt graph --format ttl      # exportez le graphe en Turtle PROV-O
```

## Comment ça marche

```
  L'agent lit un fichier de données                      L'agent calcule une valeur dérivée
            |                                                         |
            v                                                         v
  rgt hook post (hook/plugin natif)                       rgt derive --parents <ids>
            |                                                         |
            v                                      (vérification à partir des valeurs parentes)
  rgt record extrait {path, content}                                  |
            |                                                         v
            v                                                   calcul correct ?
  graphe de provenance ── rgt status ── obsolète ? ── NON ──> valeur fiable
            |                                   |
            +---------------- OUI --------------->  nœud + nœuds dépendants marqués obsolètes
```

Trois stratégies garantissent la fiabilité du graphe :

1. **Capture** : les hooks/plugins natifs transmettent chaque fichier lu par un agent à `rgt record`, si bien que les valeurs entrent dans le graphe sans que l'agent ait à y penser.
2. **Vérification** : chaque `rgt derive` applique le principe « faire confiance, mais vérifier » : RGT recalcule le résultat à partir des valeurs parentes et rejette les valeurs incorrectes avant leur insertion.
3. **Obsolescence** : quand un fichier source change, ses nœuds (et tout ce qui en est dérivé) sont marqués obsolètes, ce qui permet de demander à l'agent de relire le fichier.

## Commandes

### Initialisation et hooks
```bash
rgt init [-g] [--force] [--agent <name>]   # configure les hooks, détecte les agents installés
rgt hook pre|post [--agent <name>]         # capture passive depuis le JSON d'événement de l'agent (stdin)
```

### Provenance
```bash
rgt record <file>                          # extrait et suit les valeurs d'un fichier
rgt derive --parents <ids> --operation <op> --expression <expr> --result <val>
                                           # vérifie et enregistre une dérivation
rgt verify --parents <ids> --operation <op> --result <val>
                                           # vérifie une valeur dérivée sans l'enregistrer
rgt status [--stale-only] [--json]         # état du graphe et obsolescence
rgt query <node_id> [--json]               # lignée complète d'une valeur
rgt graph [-f text|mermaid|dot|ttl] [--include-absolute-paths] # exporte le graphe
```

Opérations : `EXPRESSION` (formules comme `a + b * c`) et `DATE_DIFF` (arithmétique de dates, ex. `date2 - date1`). Les variables parentes correspondent à `parent[0]=a, parent[1]=b, ...`.

## Outils pris en charge

RGT configure des hooks de capture de provenance pour 13 outils de codage IA, en utilisant le mécanisme natif de chaque agent :

| Outil | Installation | Méthode |
|-------|--------------|---------|
| **Claude Code** | `rgt init -g` | hook shell PreToolUse/PostToolUse (`settings.json`) |
| **Cursor** | `rgt init -g --agent cursor` | hook pre/postToolUse (`hooks.json`) |
| **GitHub Copilot (VS Code)** | `rgt init -g --agent copilot` | hooks Copilot Chat (`github.copilot.chat.hooks`) |
| **GitHub Copilot CLI** | `rgt init -g --agent copilot` | fichier d'instructions (répertoire de configuration de Copilot CLI) |
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

Valeurs `--agent` acceptées : `claude-code`, `cursor`, `codex`, `windsurf`, `copilot`, `gemini`, `vibe`, `opencode`, `pi`, `hermes`, `cline`, `antigravity`, `kilocode`, plus les alias `claude`, `roo-code`, `kilo`.

## Données et stockage

RGT stocke son graphe dans `.rgt/store.db` (SQLite) à la racine du projet ; ce fichier est créé par `rgt init`. Aucun service externe, aucune télémétrie, aucun appel réseau pendant le fonctionnement normal.

## Documentation

- **[AGENTS.md](AGENTS.md)** : instructions que RGT installe pour les agents IA (comment enregistrer et dériver)
- **[CONTRIBUTING.md](CONTRIBUTING.md)** : guide de contribution
- **[CHANGELOG.md](CHANGELOG.md)** : historique des releases

## Remerciements

RGT s'est inspiré de [RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk), un proxy CLI haute performance qui compresse la sortie shell pour les agents de codage IA. RGT suit l'approche de RTK : un binaire unique avec des intégrations de hook natives et spécifiques à chaque agent, et reprend sa couverture de 13 agents. Là où RTK filtre la sortie des commandes, RGT suit la provenance des nombres et des dates que les agents lisent et dérivent, et vérifie que chaque dérivation est mathématiquement correcte.

RGT a été développé avec l'aide des outils de codage IA de [DeepSeek](https://www.deepseek.com/).

## Contribuer

Les contributions sont les bienvenues ! Ouvrez une issue ou une PR sur [GitHub](https://github.com/rafael-bianchi/rgt).

## Licence

Sous [licence Apache, version 2.0](LICENSE).
