//! Decode generated Windows wrappers for cross-platform assertion text only.
pub fn inspect_command_text(text: &str) -> String {
    use base64::Engine;
    let regex = regex::Regex::new(r"-EncodedCommand ([A-Za-z0-9+/=]+)").unwrap();
    let mut inspected = text.to_string();
    for capture in regex.captures_iter(text) {
        let bytes = base64::prelude::BASE64_STANDARD
            .decode(&capture[1])
            .unwrap();
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        let script = String::from_utf16(&units).unwrap();
        let assignment = |key: &str| -> String {
            let tail = script.split_once(key).unwrap().1;
            tail.strip_prefix('\'')
                .unwrap()
                .split('\'')
                .next()
                .unwrap()
                .to_string()
        };
        let executable = assignment("$p.StartInfo.FileName=");
        let args = assignment("$p.StartInfo.Arguments=");
        // Keep the real wrapper and decoded script visible for deadline checks.
        let invocation = format!("'{executable}' {args}");
        inspected.push_str(&format!("\n{script}\n{invocation}\n"));
        inspected.push_str(&serde_json::to_string(&invocation).unwrap());
    }
    inspected
}

#[test]
fn decoded_windows_wrapper_retains_identity_and_deadline() {
    let command = rgt::hooks::installer::direct_hook_command_for_platform(
        std::path::Path::new("C:/tools/rgt.exe"),
        "post",
        "codex",
        true,
    );
    let inspected = inspect_command_text(&command);
    assert!(inspected.contains("'C:/tools/rgt.exe' hook post --agent codex --rgt-managed"));
    assert!(inspected.contains("ElapsedMilliseconds -lt 800"));
    assert!(inspected.contains(&command));
}
