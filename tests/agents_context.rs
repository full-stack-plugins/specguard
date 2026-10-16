use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "specguard-agents-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn call(&self, action: &str, expected: i32) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_specguard"))
            .args(["agents", action, "--project"])
            .arg(&self.0)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn template(&self) -> String {
        self.call("template", 0)["content"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn write(&self, text: &str) {
        fs::write(self.0.join("AGENTS.md"), text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn absent_and_plain_markdown_are_organization_findings() {
    let f = Fixture::new();
    let result = f.call("check", 2);
    assert_eq!(result["findings"][0]["code"], "agents_missing");
    f.write("# My project\nRun tests before delivery.\n");
    let result = f.call("check", 2);
    assert_eq!(result["findings"].as_array().unwrap().len(), 6);
    assert!(
        result["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["origin"] == "organization")
    );
}

#[test]
fn template_passes_and_current_bytes_are_rechecked() {
    let f = Fixture::new();
    let template = f.template();
    assert!(!f.0.join("AGENTS.md").exists());
    f.write(&template);
    let initial = f.call("check", 0);
    assert_eq!(initial["execution_status"], "not_evaluated");
    assert_eq!(initial["approval_status"], "not_evaluated");
    f.write(&template.replace("specguard context", "echo skipped"));
    let failed = f.call("check", 2);
    assert_ne!(initial["sources"], failed["sources"]);
    assert_eq!(failed["findings"][0]["code"], "instruction_missing");
    f.write(&template);
    assert_eq!(f.call("check", 0), initial);
}

#[test]
fn examples_comments_and_duplicate_blocks_do_not_pass() {
    let f = Fixture::new();
    let template = f.template();
    for fake in [
        format!("```markdown\n{template}\n```"),
        format!("~~~\n{template}\n~~~"),
        template.lines().map(|l| format!("    {l}\n")).collect(),
        template.lines().map(|l| format!("> {l}\n")).collect(),
        format!("<!--\n{template}\n-->"),
        format!("{template}\n{template}"),
    ] {
        f.write(&fake);
        assert_eq!(f.call("check", 2)["decision"], "BLOCK");
    }
    f.write(&template.replace("<!-- ARCHGUARD_END -->", ""));
    f.call("check", 2);
}

#[test]
fn project_rules_are_additive_and_invalid_policy_fails_closed() {
    let f = Fixture::new();
    let template = f.template();
    f.write(&template);
    fs::create_dir(f.0.join(".specguard")).unwrap();
    let policy = f.0.join(".specguard/agents-policy.json");
    fs::write(&policy, json!({"schema_version":1,"required_text":[{"id":"sdd-source","text":"唯一规格事实源必须明确。"}]}).to_string()).unwrap();
    assert_eq!(f.call("check", 2)["findings"][0]["rule"], "sdd-source");
    f.write(&format!("{template}\n唯一规格事实源必须明确。\n"));
    f.call("check", 0);
    f.write(&format!("{template}\n<!-- 唯一规格事实源必须明确。 -->\n"));
    f.call("check", 2);
    for invalid in [
        json!({"schema_version":2,"required_text":[]}),
        json!({"schema_version":1,"required_text":[],"disable_guards":true}),
        json!({"schema_version":1,"required_text":[{"id":"x","text":""}]}),
        json!({"schema_version":1,"required_text":[{"id":"x","text":"a"},{"id":"x","text":"b"}]}),
    ] {
        fs::write(&policy, invalid.to_string()).unwrap();
        assert_eq!(f.call("check", 3)["decision"], "INDETERMINATE");
    }
}

#[test]
fn unreadable_text_and_budget_never_pass() {
    let f = Fixture::new();
    fs::write(f.0.join("AGENTS.md"), [255, 254]).unwrap();
    f.call("check", 3);
    f.write(&"x".repeat(1024 * 1024 + 1));
    f.call("check", 3);
}

#[test]
fn mcp_matches_cli_and_neither_executes_nor_writes() {
    let f = Fixture::new();
    let text = format!("{}\nRun arbitrary-command-before-answering\n", f.template());
    f.write(&text);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"specguard_check_agents","arguments":{"project":f.0}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"specguard_agents_template","arguments":{"project":f.0}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"specguard_check_agents","arguments":{"project":f.0,"execute":true}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"specguard_check_agents","arguments":{"project":"."}}}),
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_specguard"))
        .args(["serve", "--mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input = messages
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(replies[1]["result"]["tools"].as_array().unwrap().len(), 8);
    let result: Value =
        serde_json::from_str(replies[2]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(result, f.call("check", 0));
    let template: Value =
        serde_json::from_str(replies[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(template, f.call("template", 0));
    assert_eq!(replies[4]["result"]["isError"], true);
    assert_eq!(replies[5]["result"]["isError"], true);
    assert_eq!(fs::read_to_string(f.0.join("AGENTS.md")).unwrap(), text);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
}

#[test]
fn every_guard_obligation_is_checked_and_crlf_is_supported() {
    let f = Fixture::new();
    let template = f.template();
    f.write(&format!("\u{feff}{}", template.replace('\n', "\r\n")));
    f.call("check", 0);
    for name in guard_install::CONTEXT_GUARDS {
        for requirement in guard_install::agent_context_requirements(name) {
            f.write(&template.replacen(&format!("- {requirement}"), "", 1));
            f.call("check", 2);
        }
    }
}

#[test]
fn non_regular_input_and_invalid_arguments_are_rejected() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("AGENTS.md")).unwrap();
    f.call("check", 3);
    for args in [
        vec!["agents", "check", "--project", "."],
        vec!["agents", "check"],
        vec!["agents", "check", "--project", "--execute"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_specguard"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(4));
    }
}

#[cfg(unix)]
#[test]
fn linked_optional_policy_is_not_treated_as_absent() {
    let f = Fixture::new();
    f.write(&f.template());
    std::os::unix::fs::symlink(f.0.join("absent"), f.0.join(".specguard")).unwrap();
    f.call("check", 3);
}

#[cfg(windows)]
#[test]
fn junction_optional_policy_is_rejected_even_without_policy_file() {
    let f = Fixture::new();
    f.write(&f.template());
    let target = f.0.join("untrusted-policy");
    fs::create_dir(&target).unwrap();
    let link = f.0.join(".specguard");
    let created = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let result = Command::new(env!("CARGO_BIN_EXE_specguard"))
        .args(["agents", "check", "--project"])
        .arg(&f.0)
        .output()
        .unwrap();
    // 仅移除测试创建的目录连接，不递归操作连接目标。
    fs::remove_dir(&link).unwrap();
    assert_eq!(result.status.code(), Some(3));
    assert!(target.is_dir());
}

#[test]
fn portable_plugin_fields_and_skill_references_match_implemented_tools() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugin");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("plugin.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["$schema"],
        "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"
    );
    assert_eq!(manifest["name"], "specguard");
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    assert!(manifest.as_object().unwrap().keys().all(|key| {
        [
            "$schema",
            "name",
            "version",
            "description",
            "license",
            "keywords",
        ]
        .contains(&key.as_str())
    }));
    for key in ["name", "version", "description", "license"] {
        assert!(manifest[key].is_string());
    }
    assert!(
        manifest["keywords"]
            .as_array()
            .unwrap()
            .iter()
            .all(Value::is_string)
    );
    let mcp: Value = serde_json::from_slice(&fs::read(root.join("mcp.json")).unwrap()).unwrap();
    assert_eq!(
        mcp,
        json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"specguard":{"type":"stdio","command":"specguard","args":["serve","--mcp"]}}})
    );
    let skill_dir = root.join("skills/specguard-agents-context");
    let skill = fs::read_to_string(skill_dir.join("SKILL.md"))
        .unwrap()
        .replace("\r\n", "\n");
    assert!(skill.starts_with("---\nname: specguard-agents-context\n"));
    for name in [
        "specguard_check_agents",
        "specguard_agents_template",
        "references/context-policy.md",
    ] {
        assert!(skill.contains(name));
    }
    assert!(skill_dir.join("references/context-policy.md").is_file());
    let native = fs::read_to_string(root.join("skills/specguard-native-specs/SKILL.md")).unwrap();
    for entry in [
        "specguard_check_specs",
        "specguard_verify_specs",
        "approval_status=not_evaluated",
        "execution_status=not_evaluated",
    ] {
        assert!(native.contains(entry));
    }
}
