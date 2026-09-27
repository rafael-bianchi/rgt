use rgt::hooks::registration::{
    client_surfaces, resolve_client_surfaces, CaptureTier, ClientSurface, Ownership,
    RegistrationHealth, RegistrationKind, RegistrationScope,
};

#[test]
fn exposes_all_thirteen_canonical_agents() {
    let names: Vec<_> = rgt::hooks::installer::AGENTS
        .iter()
        .map(|agent| agent.canonical)
        .collect();
    assert_eq!(names.len(), 13);
    for expected in [
        "claude-code",
        "cursor",
        "copilot",
        "gemini",
        "vibe",
        "opencode",
        "pi",
        "hermes",
        "codex",
        "windsurf",
        "cline",
        "antigravity",
        "kilocode",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn aliases_resolve_without_collapsing_distinct_surfaces() {
    assert_eq!(
        resolve_client_surfaces("claude")
            .iter()
            .map(|s| s.surface_id)
            .collect::<Vec<_>>(),
        vec!["claude-code"]
    );
    assert_eq!(
        resolve_client_surfaces("copilot")
            .iter()
            .map(|s| s.surface_id)
            .collect::<Vec<_>>(),
        vec!["copilot-chat", "copilot-cli"]
    );
    assert_eq!(
        resolve_client_surfaces("cline")
            .iter()
            .map(|s| s.surface_id)
            .collect::<Vec<_>>(),
        vec!["cline"]
    );
    assert_eq!(
        resolve_client_surfaces("roo-code")
            .iter()
            .map(|s| s.surface_id)
            .collect::<Vec<_>>(),
        vec!["roo-code"]
    );
    assert!(resolve_client_surfaces("unknown").is_empty());
}

#[test]
fn every_surface_has_stable_documented_read_paths_or_none() {
    assert_eq!(client_surfaces().len(), 15);
    for surface in client_surfaces() {
        for path in surface.read_paths {
            assert!(!path.id.is_empty());
            assert!(!path.tool.is_empty());
            assert!(!path.accepted_result_form.is_empty());
            assert!(!path.excluded_result_forms.is_empty());
        }
    }
    for id in [
        "cline",
        "roo-code",
        "antigravity",
        "kilocode",
        "copilot-cli",
    ] {
        let surface = ClientSurface::by_id(id).unwrap();
        assert!(
            surface.read_paths.is_empty(),
            "{id} must not invent a native read path"
        );
    }
}

#[test]
fn codex_and_windsurf_registration_are_separate_from_capture_tier() {
    let codex = ClientSurface::by_id("codex").unwrap();
    assert_eq!(codex.registration_kind, RegistrationKind::NativeHook);
    assert_eq!(codex.capture_tier, CaptureTier::ConfiguredUnverified);
    assert!(codex
        .read_paths
        .iter()
        .any(|p| p.id == "codex-post-tool-response"));

    let windsurf = ClientSurface::by_id("windsurf").unwrap();
    assert_eq!(windsurf.registration_kind, RegistrationKind::NativeHook);
    assert_eq!(windsurf.capture_tier, CaptureTier::InstructionOnly);
    let path = windsurf
        .read_paths
        .iter()
        .find(|p| p.id == "windsurf-post-read-code-path-only")
        .unwrap();
    assert_eq!(path.tool, "post_read_code");
    assert!(path.accepted_result_form.contains("file_path"));
}

#[test]
fn file_existence_never_implies_live_capture_verification() {
    let surface = ClientSurface::by_id("codex").unwrap();
    assert_eq!(surface.capture_tier, CaptureTier::ConfiguredUnverified);
    let registration = surface.local_registration(
        "/tmp/settings.json",
        RegistrationScope::User,
        true,
        Ownership::CurrentRgt,
    );
    assert_eq!(registration.health, RegistrationHealth::Active);
    assert_eq!(registration.capture_tier, CaptureTier::ConfiguredUnverified);
}
