//! Stable client-surface and local registration vocabulary.
//!
//! These descriptors describe known identities and candidate event paths. They
//! do not claim that a host loaded a registration or that an event was observed.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationKind {
    NativeHook,
    Plugin,
    Instruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CaptureTier {
    VerifiedAutomatic,
    ConfiguredUnverified,
    InstructionOnly,
    Unsupported,
    Broken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum EventPhase {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationScope {
    Project,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Ownership {
    CurrentRgt,
    RecognizedLegacyRgt,
    UserOwned,
    Ambiguous,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RegistrationHealth {
    Active,
    Missing,
    Malformed,
    Obsolete,
    Ambiguous,
    TrustPending,
    InactiveWorkspace,
    GuidanceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadPath {
    /// Stable evidence identifier for one tool and accepted result form.
    pub id: &'static str,
    /// Native host tool/event name, or a deliberately qualified unknown name.
    pub tool: &'static str,
    /// Result shape an adapter can consider after client-contract verification.
    pub accepted_result_form: &'static str,
    /// Similar-looking shapes that are specifically ineligible for values.
    pub excluded_result_forms: &'static str,
    pub phase: EventPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSurface {
    pub surface_id: &'static str,
    pub canonical_agent: &'static str,
    pub accepted_names: &'static [&'static str],
    pub registration_kind: RegistrationKind,
    pub capture_tier: CaptureTier,
    pub read_paths: &'static [ReadPath],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationRegistration {
    pub surface_id: &'static str,
    pub registration_kind: RegistrationKind,
    pub scope: RegistrationScope,
    pub artifact_path: PathBuf,
    pub entrypoint: String,
    pub event_phases: Vec<EventPhase>,
    pub ownership: Ownership,
    pub health: RegistrationHealth,
    /// Live capture support is deliberately independent from local health.
    pub capture_tier: CaptureTier,
}

/// Health of a documented local instruction path. This says nothing about
/// whether the client loaded the file or followed its instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidanceHealth {
    Usable,
    /// Readable instructions without the installer closing marker; safe to
    /// follow locally, but RGT cannot claim safe ownership for rewrites.
    UsableUnmanaged,
    Missing,
    Incomplete,
    Unowned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidanceInspection {
    pub surface_id: &'static str,
    pub scope: RegistrationScope,
    pub artifact_path: PathBuf,
    pub health: GuidanceHealth,
}

const CLAUDE_PATHS: &[ReadPath] = &[
    ReadPath {
        id: "claude-read-post-content",
        tool: "Read",
        accepted_result_form: "PostToolUse tool_response.content containing the complete decoded file text",
        excluded_result_forms: "PreToolUse; missing content; truncated or filtered content; non-Read tools; a file path without returned text",
        phase: EventPhase::After,
    },
    ReadPath {
        id: "claude-bash-read-stdout",
        tool: "Bash",
        accepted_result_form: "PostToolUse stdout from a recognized read command whose full text matches the current file snapshot",
        excluded_result_forms: "PreToolUse; non-read commands; missing, truncated, or partial stdout; path without returned text",
        phase: EventPhase::After,
    },
];

const CODEX_PATHS: &[ReadPath] = &[ReadPath {
    id: "codex-post-tool-response",
    tool: "file-read tool (exact host tool name is client-version dependent)",
    accepted_result_form: "PostToolUse tool_response containing complete, untruncated file bytes/text that match the current snapshot",
    excluded_result_forms: "PreToolUse; absent, non-file, truncated, partial, filtered, or path-only tool_response",
    phase: EventPhase::After,
}];

const OPENCODE_PATHS: &[ReadPath] = &[ReadPath {
    id: "opencode-read-display-text",
    tool: "read",
    accepted_result_form: "tool.execute.after read with a full, untruncated metadata.display.text result that exactly matches the source snapshot",
    excluded_result_forms: "before callbacks; other tools; missing, partial, or truncated display; failed reads; changed source bytes; newline-terminated files when display.text omits the final newline",
    phase: EventPhase::After,
}];

const WINDSURF_PATHS: &[ReadPath] = &[ReadPath {
    id: "windsurf-post-read-code-path-only",
    tool: "post_read_code",
    accepted_result_form: "post_read_code event with file_path, for hook-invocation evidence only",
    excluded_result_forms:
        "All value capture: the documented event has no completed source content",
    phase: EventPhase::After,
}];

const SURFACES: &[ClientSurface] = &[
    ClientSurface {
        surface_id: "claude-code",
        canonical_agent: "claude-code",
        accepted_names: &["claude-code", "claude"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: CLAUDE_PATHS,
    },
    ClientSurface {
        surface_id: "cursor",
        canonical_agent: "cursor",
        accepted_names: &["cursor"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "copilot-chat",
        canonical_agent: "copilot",
        accepted_names: &["copilot", "copilot-chat"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "copilot-cli",
        canonical_agent: "copilot",
        accepted_names: &["copilot", "copilot-cli"],
        registration_kind: RegistrationKind::Instruction,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "gemini",
        canonical_agent: "gemini",
        accepted_names: &["gemini"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "vibe",
        canonical_agent: "vibe",
        accepted_names: &["vibe"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "opencode",
        canonical_agent: "opencode",
        accepted_names: &["opencode"],
        registration_kind: RegistrationKind::Plugin,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: OPENCODE_PATHS,
    },
    ClientSurface {
        surface_id: "pi",
        canonical_agent: "pi",
        accepted_names: &["pi"],
        registration_kind: RegistrationKind::Plugin,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "hermes",
        canonical_agent: "hermes",
        accepted_names: &["hermes"],
        registration_kind: RegistrationKind::Plugin,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "codex",
        canonical_agent: "codex",
        accepted_names: &["codex"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::ConfiguredUnverified,
        read_paths: CODEX_PATHS,
    },
    ClientSurface {
        surface_id: "windsurf",
        canonical_agent: "windsurf",
        accepted_names: &["windsurf"],
        registration_kind: RegistrationKind::NativeHook,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: WINDSURF_PATHS,
    },
    ClientSurface {
        surface_id: "cline",
        canonical_agent: "cline",
        accepted_names: &["cline"],
        registration_kind: RegistrationKind::Instruction,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "roo-code",
        canonical_agent: "cline",
        accepted_names: &["roo-code"],
        registration_kind: RegistrationKind::Instruction,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "antigravity",
        canonical_agent: "antigravity",
        accepted_names: &["antigravity"],
        registration_kind: RegistrationKind::Instruction,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
    ClientSurface {
        surface_id: "kilocode",
        canonical_agent: "kilocode",
        accepted_names: &["kilocode", "kilo"],
        registration_kind: RegistrationKind::Instruction,
        capture_tier: CaptureTier::InstructionOnly,
        read_paths: &[],
    },
];

pub fn client_surfaces() -> &'static [ClientSurface] {
    SURFACES
}

impl ClientSurface {
    #[allow(dead_code)]
    pub fn by_id(id: &str) -> Option<&'static ClientSurface> {
        SURFACES.iter().find(|surface| surface.surface_id == id)
    }

    /// Constructs a local observation. `entry_present` means an RGT entry was
    /// independently identified by an inspector; file existence alone must
    /// never be passed as true.
    pub fn local_registration(
        &self,
        artifact_path: impl AsRef<Path>,
        scope: RegistrationScope,
        entry_present: bool,
        ownership: Ownership,
    ) -> IntegrationRegistration {
        let health = if self.registration_kind == RegistrationKind::Instruction {
            if entry_present {
                RegistrationHealth::GuidanceOnly
            } else {
                RegistrationHealth::Missing
            }
        } else if entry_present {
            RegistrationHealth::Active
        } else {
            RegistrationHealth::Missing
        };
        IntegrationRegistration {
            surface_id: self.surface_id,
            registration_kind: self.registration_kind,
            scope,
            artifact_path: artifact_path.as_ref().to_path_buf(),
            entrypoint: format!("rgt hook --agent {}", self.canonical_agent),
            event_phases: self.read_paths.iter().map(|path| path.phase).collect(),
            ownership,
            health,
            capture_tier: self.capture_tier,
        }
    }
}

/// Resolve canonical IDs and aliases to concrete surfaces. A grouped name such
/// as `copilot` returns both Chat and CLI, preserving their separate evidence.
pub fn resolve_client_surfaces(name: &str) -> Vec<&'static ClientSurface> {
    SURFACES
        .iter()
        .filter(|surface| surface.accepted_names.contains(&name))
        .collect()
}

/// Inspects only local artifacts for every client surface. It never infers
/// that a client loaded or invoked an artifact. The returned tier is always
/// independent from filesystem health.
pub fn inspect_local_registrations(home: &Path, config_dir: &Path) -> Vec<IntegrationRegistration> {
    client_surfaces()
        .iter()
        .map(|surface| inspect_surface(surface, home, config_dir))
        .collect()
}

/// Inspect each surface with a documented local instruction path separately
/// from its native hook or plugin registration.
pub fn inspect_local_guidance(home: &Path, config_dir: &Path) -> Vec<GuidanceInspection> {
    client_surfaces()
        .iter()
        .filter_map(|surface| {
            let candidates = guidance_candidates(surface.surface_id, home, config_dir);
            let mut fallback = None;
            for (path, scope) in candidates {
                let health = match fs::read_to_string(&path) {
                    Ok(text) => guidance_health(&text, surface.surface_id),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        GuidanceHealth::Missing
                    }
                    Err(_) => GuidanceHealth::Incomplete,
                };
                let inspection = GuidanceInspection {
                    surface_id: surface.surface_id,
                    scope,
                    artifact_path: path,
                    health,
                };
                if health == GuidanceHealth::Usable {
                    return Some(inspection);
                }
                if fallback.as_ref().is_none_or(|prior: &GuidanceInspection| {
                    guidance_priority(health) > guidance_priority(prior.health)
                }) {
                    fallback = Some(inspection);
                }
            }
            fallback
        })
        .collect()
}

fn guidance_priority(health: GuidanceHealth) -> u8 {
    match health {
        GuidanceHealth::Usable => 4,
        GuidanceHealth::UsableUnmanaged => 3,
        GuidanceHealth::Incomplete => 2,
        GuidanceHealth::Unowned => 1,
        GuidanceHealth::Missing => 0,
    }
}

fn guidance_health(text: &str, surface_id: &str) -> GuidanceHealth {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    let starts: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            (*line == crate::hooks::glue::RGT_MARKER || *line == "# RGT Integration")
                .then_some(index)
        })
        .collect();
    if starts.is_empty() {
        return if lines.contains(&crate::hooks::glue::RGT_END_MARKER) {
            GuidanceHealth::Incomplete
        } else {
            GuidanceHealth::Unowned
        };
    }
    let ends: Vec<_> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (*line == crate::hooks::glue::RGT_END_MARKER).then_some(index))
        .collect();
    if starts.len() != 1 || ends.len() > 1 || ends.first().is_some_and(|end| *end <= starts[0]) {
        return GuidanceHealth::Incomplete;
    }
    let body_end = ends.first().copied().unwrap_or(lines.len());
    let body = &lines[starts[0] + 1..body_end];
    if !body.iter().any(|line| line.contains("rgt record <file>")) {
        return GuidanceHealth::Incomplete;
    }
    if surface_id == "cursor" {
        let frontmatter = &lines[..starts[0]];
        let frontmatter_end = frontmatter.iter().skip(1).position(|line| *line == "---");
        let Some(frontmatter_end) = frontmatter_end.map(|index| index + 1) else {
            return GuidanceHealth::Incomplete;
        };
        let settings: Vec<_> = frontmatter[1..frontmatter_end]
            .iter()
            .filter_map(|line| {
                line.split_once(':')
                    .filter(|(key, _)| key.trim() == "alwaysApply")
                    .map(|(_, value)| value.trim())
            })
            .collect();
        if frontmatter.first() != Some(&"---") || settings.as_slice() != ["true"] {
            return GuidanceHealth::Incomplete;
        }
    }
    if ends.is_empty() {
        GuidanceHealth::UsableUnmanaged
    } else {
        GuidanceHealth::Usable
    }
}

fn guidance_candidates(
    surface_id: &str,
    home: &Path,
    config_dir: &Path,
) -> Vec<(PathBuf, RegistrationScope)> {
    use RegistrationScope::{Project, User};
    let project = |path: &str| (PathBuf::from(path), Project);
    let user = |path: PathBuf| (path, User);
    match surface_id {
        "claude-code" => vec![project("CLAUDE.md"), user(home.join(".claude/CLAUDE.md"))],
        "cursor" => vec![project(".cursor/rules/rgt.mdc")],
        "copilot-chat" => vec![project(".github/copilot-instructions.md")],
        "copilot-cli" => vec![user(
            crate::hooks::paths::copilot_cli_config_dir(home, config_dir).join("AGENTS.md"),
        )],
        "gemini" => vec![project("GEMINI.md"), user(home.join(".gemini/GEMINI.md"))],
        "vibe" => vec![project("AGENTS.md"), user(home.join(".vibe/AGENTS.md"))],
        "pi" => vec![
            project(".pi/APPEND_SYSTEM.md"),
            user(crate::hooks::paths::pi_guidance_path(home, true)),
        ],
        "hermes" => vec![(crate::hooks::paths::hermes_guidance_path(), Project)],
        "codex" => vec![project("AGENTS.md")],
        "windsurf" => vec![project(".windsurfrules")],
        "cline" | "roo-code" => vec![project(".clinerules")],
        "antigravity" => vec![project(".agents/rules/antigravity-rgt-rules.md")],
        "kilocode" => vec![project(".kilocode/rules/rgt-rules.md")],
        _ => Vec::new(),
    }
}

fn inspect_surface(
    surface: &ClientSurface,
    home: &Path,
    config_dir: &Path,
) -> IntegrationRegistration {
    let candidates = artifact_candidates(surface.surface_id, home, config_dir);
    let mut matched = Vec::new();
    let mut malformed = None;
    let mut existing = None;
    for (path, scope) in &candidates {
        if !path.exists() {
            continue;
        }
        existing.get_or_insert((path.clone(), *scope));
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(_) => {
                malformed = Some((path.clone(), *scope));
                continue;
            }
        };
        match inspect_artifact(surface, path, &text) {
            Ok(Some((mut health, ownership, phases))) => {
                if surface.surface_id == "hermes" && health == RegistrationHealth::Active {
                    health = hermes_enablement_health(*scope, home);
                }
                matched.push((path.clone(), *scope, health, ownership, phases));
            }
            Ok(None) => {}
            Err(()) => malformed = Some((path.clone(), *scope)),
        }
    }

    if matched.len() > 1 {
        let (path, scope, _, _, _) = matched.remove(0);
        let mut registration = surface.local_registration(path, scope, true, Ownership::Ambiguous);
        registration.health = RegistrationHealth::Ambiguous;
        return registration;
    }
    if !matched.is_empty() && malformed.is_some() {
        let (path, scope, _, _, phases) = matched.remove(0);
        let mut registration = surface.local_registration(path, scope, true, Ownership::Ambiguous);
        registration.health = RegistrationHealth::Ambiguous;
        registration.event_phases = phases;
        return registration;
    }
    if let Some((path, scope, health, ownership, phases)) = matched.pop() {
        let mut registration = surface.local_registration(path, scope, true, ownership);
        registration.health = health;
        registration.event_phases = phases;
        return registration;
    }
    let had_malformed = malformed.is_some();
    let had_existing = existing.is_some();
    let selected = malformed
        .or(existing)
        .or_else(|| candidates.first().cloned());
    let (artifact_path, scope) =
        selected.unwrap_or_else(|| (PathBuf::new(), RegistrationScope::Project));
    let mut registration = surface.local_registration(
        &artifact_path,
        scope,
        false,
        if had_existing {
            Ownership::UserOwned
        } else {
            Ownership::Unknown
        },
    );
    if had_malformed {
        registration.health = RegistrationHealth::Malformed;
    }
    registration
}

fn hermes_enablement_health(scope: RegistrationScope, home: &Path) -> RegistrationHealth {
    let config_path = match scope {
        RegistrationScope::Project => PathBuf::from(".hermes/config.toml"),
        RegistrationScope::User => home.join(".hermes/config.toml"),
    };
    let content = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return RegistrationHealth::Obsolete;
        }
        Err(_) => return RegistrationHealth::Malformed,
    };
    let doc = match content.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => doc,
        Err(_) => return RegistrationHealth::Malformed,
    };
    let Some(plugins) = doc.get("plugins") else {
        return RegistrationHealth::Obsolete;
    };
    let enabled = if let Some(table) = plugins.as_table() {
        table.get("enabled").map(toml_edit::Item::as_array)
    } else if let Some(inline) = plugins
        .as_value()
        .and_then(toml_edit::Value::as_inline_table)
    {
        inline.get("enabled").map(toml_edit::Value::as_array)
    } else {
        return RegistrationHealth::Malformed;
    };
    let Some(enabled) = enabled else {
        return RegistrationHealth::Obsolete;
    };
    let Some(enabled) = enabled else {
        return RegistrationHealth::Malformed;
    };
    if enabled.iter().any(|value| value.as_str().is_none()) {
        return RegistrationHealth::Malformed;
    }
    if enabled.iter().any(|value| value.as_str() == Some("rgt")) {
        RegistrationHealth::Active
    } else {
        RegistrationHealth::Obsolete
    }
}

fn inspect_artifact(
    surface: &ClientSurface,
    path: &Path,
    text: &str,
) -> Result<Option<(RegistrationHealth, Ownership, Vec<EventPhase>)>, ()> {
    match surface.registration_kind {
        RegistrationKind::Instruction => match guidance_health(text, surface.surface_id) {
            GuidanceHealth::Usable => Ok(Some((
                RegistrationHealth::GuidanceOnly,
                Ownership::CurrentRgt,
                Vec::new(),
            ))),
            GuidanceHealth::UsableUnmanaged => Ok(Some((
                RegistrationHealth::GuidanceOnly,
                Ownership::Ambiguous,
                Vec::new(),
            ))),
            GuidanceHealth::Missing | GuidanceHealth::Unowned => Ok(None),
            GuidanceHealth::Incomplete => Ok(Some((
                RegistrationHealth::Malformed,
                Ownership::CurrentRgt,
                Vec::new(),
            ))),
        },
        RegistrationKind::NativeHook => inspect_native_hook(surface, path, text),
        RegistrationKind::Plugin => inspect_plugin(surface, path, text),
    }
}

fn inspect_native_hook(
    surface: &ClientSurface,
    path: &Path,
    text: &str,
) -> Result<Option<(RegistrationHealth, Ownership, Vec<EventPhase>)>, ()> {
    let callbacks = if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        let expected_hook_path = match surface.surface_id {
            "copilot-chat" => "github.copilot.chat.hooks",
            "claude-code" | "cursor" | "codex" | "windsurf" => "hooks",
            _ => "",
        };
        crate::hooks::editor::jsonc_hook_callbacks(text)
            .map_err(|_| ())?
            .into_iter()
            .map(|callback| {
                let shape_ok = match surface.surface_id {
                    "claude-code" | "copilot-chat" | "codex" => {
                        callback.nested_callback_group
                            && callback.callback_type.as_deref() == Some("command")
                    }
                    "cursor" | "windsurf" => !callback.nested_callback_group,
                    _ => false,
                };
                NativeHookCallback {
                    event: callback.event,
                    command: callback.command,
                    group_valid: callback.hook_path == expected_hook_path
                        && callback.callback_count == 1
                        && shape_ok,
                }
            })
            .collect::<Vec<_>>()
    } else if path
        .extension()
        .is_some_and(|extension| extension == "toml")
    {
        let doc = text.parse::<toml_edit::DocumentMut>().map_err(|_| ())?;
        match surface.surface_id {
            "gemini" => doc
                .get("PostToolUse")
                .and_then(toml_edit::Item::as_table)
                .and_then(|table| table.get("command"))
                .and_then(toml_edit::Item::as_str)
                .map(|command| {
                    vec![NativeHookCallback {
                        event: "PostToolUse".to_string(),
                        command: command.to_string(),
                        group_valid: true,
                    }]
                })
                .unwrap_or_default(),
            "vibe" => {
                let mut callbacks = Vec::new();
                if let Some(entries) = doc.get("pre_tool") {
                    if let Some(tables) = entries.as_array_of_tables() {
                        callbacks.extend(tables.iter().filter_map(|table| {
                            table
                                .get("command")
                                .and_then(toml_edit::Item::as_str)
                                .map(|command| NativeHookCallback {
                                    event: "pre_tool".to_string(),
                                    command: command.to_string(),
                                    group_valid: true,
                                })
                        }));
                    } else if let Some(array) = entries.as_array() {
                        callbacks.extend(array.iter().filter_map(|value| {
                            value
                                .as_inline_table()
                                .and_then(|table| table.get("command"))
                                .and_then(toml_edit::Value::as_str)
                                .map(|command| NativeHookCallback {
                                    event: "pre_tool".to_string(),
                                    command: command.to_string(),
                                    group_valid: true,
                                })
                        }));
                    }
                }
                callbacks
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let rgt_commands: Vec<_> = callbacks
        .iter()
        .filter(|callback| crate::hooks::editor::is_rgt_command(&callback.command))
        .collect();
    if rgt_commands.is_empty() {
        return Ok(None);
    }

    let mut phases = Vec::new();
    let mut matching_post = 0;
    let mut matching_pre = 0;
    let mut current_marker = true;
    let mut wrong_identity = false;
    let mut resolvable = true;
    for callback in rgt_commands {
        let identity = crate::hooks::editor::rgt_hook_identity(&callback.command);
        if let Some((agent, event)) = identity {
            if agent != surface.canonical_agent {
                wrong_identity = true;
            } else {
                if !callback.group_valid {
                    wrong_identity = true;
                }
                match expected_host_phase(surface.surface_id, &callback.event) {
                    Some((phase, expected_event)) if event == expected_event => {
                        match phase {
                            EventPhase::After => matching_post += 1,
                            EventPhase::Before => matching_pre += 1,
                        }
                        if !phases.contains(&phase) {
                            phases.push(phase);
                        }
                    }
                    _ => wrong_identity = true,
                }
            }
        } else {
            wrong_identity = true;
        }
        if !callback.command.contains("--rgt-managed") {
            current_marker = false;
        }
        resolvable &= crate::hooks::editor::hook_command_executable_resolvable(&callback.command);
        if callback.event.starts_with("pre") && !phases.contains(&EventPhase::Before) {
            phases.push(EventPhase::Before);
        } else if !callback.event.starts_with("pre") && !phases.contains(&EventPhase::After) {
            phases.push(EventPhase::After);
        }
    }
    let has_duplicate = matching_post > 1 || matching_pre > 1;
    let required_hook_present =
        matching_post == 1 || (surface.surface_id == "vibe" && matching_pre == 1);
    if has_duplicate {
        return Ok(Some((
            RegistrationHealth::Ambiguous,
            Ownership::Ambiguous,
            phases,
        )));
    }
    if wrong_identity || !required_hook_present || !resolvable || !current_marker {
        return Ok(Some((
            RegistrationHealth::Obsolete,
            if current_marker {
                Ownership::CurrentRgt
            } else {
                Ownership::RecognizedLegacyRgt
            },
            phases,
        )));
    }
    Ok(Some((
        RegistrationHealth::Active,
        Ownership::CurrentRgt,
        phases,
    )))
}

#[derive(Debug)]
struct NativeHookCallback {
    event: String,
    command: String,
    group_valid: bool,
}

fn expected_host_phase(surface: &str, event: &str) -> Option<(EventPhase, &'static str)> {
    match (surface, event) {
        ("claude-code", "PostToolUse")
        | ("copilot-chat", "PostToolUse")
        | ("codex", "PostToolUse")
        | ("gemini", "PostToolUse") => Some((EventPhase::After, "post")),
        ("claude-code", "PreToolUse") => Some((EventPhase::Before, "pre")),
        ("cursor", "postToolUse") => Some((EventPhase::After, "post")),
        ("cursor", "preToolUse") => Some((EventPhase::Before, "pre")),
        ("windsurf", "post_read_code") => Some((EventPhase::After, "post")),
        ("vibe", "pre_tool") => Some((EventPhase::Before, "pre")),
        _ => None,
    }
}

fn inspect_plugin(
    surface: &ClientSurface,
    path: &Path,
    text: &str,
) -> Result<Option<(RegistrationHealth, Ownership, Vec<EventPhase>)>, ()> {
    let current_marker = text.contains("RGT-managed integration.");
    let expected_args = match surface.surface_id {
        "opencode" => "[\"hook\", \"post\", \"--agent\", \"opencode\", \"--rgt-managed\"]",
        "pi" => "[\"hook\", \"post\", \"--agent\", \"pi\", \"--rgt-managed\"]",
        "hermes" => "\"hook\", \"post\", \"--agent\", \"hermes\", \"--rgt-managed\"",
        _ => return Ok(None),
    };
    let entrypoint_matches = match surface.surface_id {
        "opencode" => {
            text.contains("export const RgtPlugin = async")
                && text.contains("\"tool.execute.after\": async (input, output)")
                && text.contains("spawnSync(")
        }
        "pi" => {
            text.contains("export async function onToolCall(input)") && text.contains("spawnSync(")
        }
        "hermes" => {
            text.contains("def post_tool_call(args):\n    _capture(args)")
                && text.contains("subprocess.run(")
        }
        _ => false,
    };
    let argument_shape_matches = text.contains(expected_args);
    let executable = if entrypoint_matches && argument_shape_matches {
        plugin_executable(surface.surface_id, text)
    } else {
        None
    };
    let recognized_invocation =
        entrypoint_matches && argument_shape_matches && executable.is_some();
    if !current_marker && !recognized_invocation {
        return Ok(None);
    }

    let executable_resolvable = executable
        .as_deref()
        .is_some_and(local_executable_resolvable);
    let active = current_marker
        && recognized_invocation
        && executable_resolvable
        && plugin_expected_callback_is_after(surface.surface_id)
        && !(surface.surface_id == "opencode" && path.ends_with(".opencode/plugin/rgt.ts"));
    let ownership = if current_marker {
        Ownership::CurrentRgt
    } else {
        Ownership::RecognizedLegacyRgt
    };
    Ok(Some((
        if active {
            RegistrationHealth::Active
        } else {
            RegistrationHealth::Obsolete
        },
        ownership,
        if recognized_invocation {
            vec![EventPhase::After]
        } else {
            Vec::new()
        },
    )))
}

fn plugin_expected_callback_is_after(surface: &str) -> bool {
    matches!(surface, "opencode" | "pi" | "hermes")
}

fn plugin_executable(surface: &str, text: &str) -> Option<String> {
    match surface {
        "opencode" | "pi" => {
            let invocation = text.get(text.find("spawnSync(")? + "spawnSync(".len()..)?;
            parse_double_quoted_string(invocation.trim_start())
        }
        "hermes" => {
            let invocation = text.get(text.find("subprocess.run(")? + "subprocess.run(".len()..)?;
            let array = invocation.find('[')?;
            parse_double_quoted_string(invocation.get(array + 1..)?.trim_start())
        }
        _ => None,
    }
}

fn parse_double_quoted_string(text: &str) -> Option<String> {
    let mut chars = text.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut output = String::new();
    let mut escaped = false;
    for character in chars {
        if escaped {
            output.push(character);
            escaped = false;
        } else {
            match character {
                '\\' => escaped = true,
                '"' => return Some(output),
                value => output.push(value),
            }
        }
    }
    None
}

fn local_executable_resolvable(executable: &str) -> bool {
    let path = Path::new(executable);
    if path.is_absolute() {
        return path.is_file();
    }
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| {
            directory.join(executable).is_file()
                || (cfg!(windows) && directory.join(format!("{executable}.exe")).is_file())
        })
    })
}

fn artifact_candidates(
    surface_id: &str,
    home: &Path,
    config_dir: &Path,
) -> Vec<(PathBuf, RegistrationScope)> {
    use RegistrationScope::{Project, User};
    let project = |path: &str| (PathBuf::from(path), Project);
    let user = |path: PathBuf| (path, User);
    match surface_id {
        "claude-code" => vec![
            project(".claude/settings.json"),
            user(home.join(".claude/settings.json")),
        ],
        "cursor" => vec![
            project(".cursor/hooks.json"),
            user(home.join(".cursor/hooks.json")),
        ],
        "copilot-chat" => vec![user(crate::hooks::paths::copilot_user_settings(config_dir))],
        "copilot-cli" => vec![user(
            crate::hooks::paths::copilot_cli_config_dir(home, config_dir).join("AGENTS.md"),
        )],
        "gemini" => vec![user(home.join(".gemini/hooks.toml"))],
        "vibe" => vec![user(home.join(".vibe/hooks.toml"))],
        "opencode" => vec![
            project(".opencode/plugins/rgt.ts"),
            project(".opencode/plugin/rgt.ts"),
            user(home.join(".config/opencode/plugins/rgt.ts")),
        ],
        "pi" => vec![
            project(".pi/extensions/rgt.ts"),
            user(home.join(".pi/agent/extensions/rgt.ts")),
        ],
        "hermes" => vec![
            project(".hermes/plugins/rgt/plugin.py"),
            user(home.join(".hermes/plugins/rgt/plugin.py")),
        ],
        "codex" => vec![
            (crate::hooks::paths::codex_hooks_json(home, false), Project),
            user(crate::hooks::paths::codex_hooks_json(home, true)),
        ],
        "windsurf" => windsurf_candidates(home),
        "cline" | "roo-code" => vec![project(".clinerules")],
        "antigravity" => vec![project(".agents/rules/antigravity-rgt-rules.md")],
        "kilocode" => vec![project(".kilocode/rules/rgt-rules.md")],
        _ => Vec::new(),
    }
}

fn windsurf_candidates(home: &Path) -> Vec<(PathBuf, RegistrationScope)> {
    use RegistrationScope::{Project, User};
    let preferred = crate::hooks::paths::windsurf_workspace_hooks_json();
    let legacy = crate::hooks::paths::windsurf_legacy_workspace_hooks_json();
    let selected_project = if preferred.exists() {
        match fs::read_to_string(&preferred)
            .map_err(|_| ())
            .and_then(|text| crate::hooks::editor::jsonc_has_active_hooks(&text).map_err(|_| ()))
        {
            Ok(false) if legacy.exists() => legacy,
            _ => preferred,
        }
    } else if legacy.exists() {
        legacy
    } else {
        preferred
    };
    vec![
        (selected_project, Project),
        (crate::hooks::paths::windsurf_user_hooks_json(home), User),
    ]
}
