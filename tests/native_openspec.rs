mod common;
use common::*;
use specguard::{graph::build_graph, model::*, parser::parse, source::*};
const MAIN: &str = include_str!("../fixtures/source-versions/openspec-1.14.1/main/spec.md");
const ADDED: &str =
    include_str!("../fixtures/source-versions/openspec-1.14.1/change/specs/session/spec.md");
const IDS: &str = include_str!("../fixtures/source-versions/openspec-1.14.1/identities.json");
fn native_snapshot(format: &str, text: &[u8], ids: &[u8]) -> (tempfile::TempDir, SourceSnapshot) {
    let d = repo();
    std::fs::create_dir(d.path().join("specs")).unwrap();
    std::fs::write(d.path().join("specs/a.md"), text).unwrap();
    std::fs::write(d.path().join("ids.json"), ids).unwrap();
    let mut config = policy();
    config.roots[0].format = format.into();
    config.roots.push(SourceRoot {
        path: "ids.json".into(),
        format: "openspec-identities/v1".into(),
        namespace: "demo".into(),
        authority: "primary".into(),
    });
    let inventory = discover(d.path(), &config).unwrap();
    let s = freeze(d.path(), &inventory, binding(d.path())).unwrap();
    (d, s)
}
#[test]
fn actual_main_and_added_formats_use_frozen_explicit_ids() {
    for (format, text) in [
        ("openspec/1.14.1-main", MAIN),
        ("openspec/1.14.1-added", ADDED),
    ] {
        let (d, s) = native_snapshot(format, text.as_bytes(), IDS.as_bytes());
        let p = parse(&s);
        assert!(
            build_graph(p.clone()).complete(),
            "{format}: {:?}",
            p.sources
        );
        assert_eq!(p.requirements[0].key, key("R1"));
        assert_eq!(p.acceptances.len(), 2);
        assert_eq!(p.acceptances[0].key, key("A1"));
        assert_eq!(p.requirements[0].source.line, 8);
        assert_eq!(
            std::fs::read(d.path().join("specs/a.md")).unwrap(),
            text.as_bytes()
        );
    }
}
#[test]
fn native_headings_are_not_inferred_as_permanent_identity() {
    let (_, valid) = native_snapshot("openspec/1.14.1-main", MAIN.as_bytes(), IDS.as_bytes());
    assert!(build_graph(parse(&valid)).complete());
    for ids in [
        "{}".to_string(),
        IDS.replace("Password authentication", "Different title"),
        IDS.replace("A2", "A1"),
        IDS.replace("specguard.openspec-ids/v1", "unknown/v9"),
    ] {
        let (_, s) = native_snapshot("openspec/1.14.1-main", MAIN.as_bytes(), ids.as_bytes());
        let parsed = parse(&s);
        assert!(!build_graph(parsed.clone()).complete());
        assert!(parsed.requirements.is_empty());
    }
}
#[test]
fn fences_are_data_and_unknown_delta_operations_fail_closed() {
    let text=MAIN.replace("#### Scenario: Correct password","```markdown\n### Requirement: Fake\n#### Scenario: Fake\n```\n\n#### Scenario: Correct password");
    let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let p = parse(&s);
    assert!(build_graph(p.clone()).complete(), "{:?}", p.sources);
    assert_eq!(p.requirements.len(), 1);
    for operation in ["MODIFIED", "REMOVED", "RENAMED"] {
        let text = ADDED.replace("ADDED", operation);
        let (_, s) = native_snapshot("openspec/1.14.1-added", text.as_bytes(), IDS.as_bytes());
        assert!(
            parse(&s)
                .sources
                .iter()
                .any(|s| s.status == Terminal::Unsupported)
        );
    }
}
#[test]
fn native_ast_depth_and_bad_encoding_are_incomplete() {
    let (d, s) = native_snapshot("openspec/1.14.1-main", MAIN.as_bytes(), IDS.as_bytes());
    assert!(build_graph(parse(&s)).complete());
    let mut inventory = s.inventory;
    inventory.limits.max_depth = 1;
    let limited = freeze(d.path(), &inventory, s.binding).unwrap();
    assert!(
        parse(&limited)
            .sources
            .iter()
            .any(|s| s.status == Terminal::Limit)
    );
    let (_, s) = native_snapshot("openspec/1.14.1-main", &[255], IDS.as_bytes());
    assert!(!build_graph(parse(&s)).complete());
}

#[test]
fn extraction_matches_official_1_14_1_and_normalizes_crlf_bom_closed_headers() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/source-versions/openspec-1.14.1/official-extraction.json"
    ))
    .unwrap();
    let text = format!(
        "\u{feff}{}",
        MAIN.replace("Password authentication\n", "Password authentication ###\n")
            .replace("Correct password\n", "Correct password ####\n")
            .replace('\n', "\r\n")
    );
    let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let p = parse(&s);
    assert!(build_graph(p.clone()).complete(), "{:?}", p.sources);
    assert_eq!(
        p.requirements[0].text,
        expected["requirement"]["text"].as_str().unwrap()
    );
    assert_eq!(
        p.acceptances[0].text,
        expected["requirement"]["scenarios"][0]["rawText"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        p.acceptances[1].text,
        expected["requirement"]["scenarios"][1]["rawText"]
            .as_str()
            .unwrap()
    );
}

#[test]
fn malformed_native_structure_never_produces_a_complete_subset() {
    let (_, s) = native_snapshot("openspec/1.14.1-main", MAIN.as_bytes(), IDS.as_bytes());
    assert!(build_graph(parse(&s)).complete());
    for text in [
        MAIN.replace("## Requirements", "## Other"),
        MAIN.replace(" SHALL ", " should "),
        MAIN.replace("## Purpose", "## Context"),
        MAIN.replace(
            "#### Scenario: Incorrect password",
            "#### Scenario: Unknown",
        ),
        format!(
            "{MAIN}\n### Requirement: Password authentication\nThe app MUST reject invalid input.\n"
        ),
        ADDED.to_owned(),
    ] {
        let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
        let p = parse(&s);
        assert!(!build_graph(p.clone()).complete(), "{text}");
        assert!(p.requirements.is_empty());
    }
}

#[test]
fn unsupported_html_structure_is_not_silently_ignored() {
    let text=MAIN.replace("### Requirement: Password authentication","<!--\n### Requirement: Hidden\nThe app MUST enforce a hidden requirement.\n-->\n\n### Requirement: Password authentication");
    let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let p = parse(&s);
    assert!(p.sources.iter().any(|s| s.status == Terminal::Unsupported));
    assert!(!build_graph(p).complete());
}

#[test]
fn normalized_carriage_return_lines_obey_line_budget() {
    let text = MAIN.replace('\n', "\r");
    let (d, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let mut inventory = s.inventory;
    inventory.limits.max_lines = 14;
    let s = freeze(d.path(), &inventory, s.binding).unwrap();
    assert!(
        parse(&s)
            .sources
            .iter()
            .any(|s| s.path == "specs/a.md" && s.status == Terminal::Limit)
    );
}

#[test]
fn native_identity_registry_is_part_of_source_digest_and_renames_are_explicit() {
    let (d, s) = native_snapshot("openspec/1.14.1-main", MAIN.as_bytes(), IDS.as_bytes());
    let original = parse(&s);
    let title = "Password-based login";
    std::fs::write(
        d.path().join("specs/a.md"),
        MAIN.replace("Password authentication", title),
    )
    .unwrap();
    std::fs::write(
        d.path().join("ids.json"),
        IDS.replace("Password authentication", title),
    )
    .unwrap();
    let mut config = policy();
    config.roots[0].format = "openspec/1.14.1-main".into();
    config.roots.push(SourceRoot {
        path: "ids.json".into(),
        format: "openspec-identities/v1".into(),
        namespace: "demo".into(),
        authority: "primary".into(),
    });
    let new = freeze(
        d.path(),
        &discover(d.path(), &config).unwrap(),
        s.binding.clone(),
    )
    .unwrap();
    let parsed = parse(&new);
    assert!(build_graph(parsed.clone()).complete());
    assert_eq!(original.requirements[0].key, parsed.requirements[0].key);
    assert_ne!(s.digest, new.digest);
    std::fs::write(
        d.path().join("ids.json"),
        IDS.replace("Password authentication", title)
            .replace("\"R1\"", "\"R2\""),
    )
    .unwrap();
    let changed_ids = freeze(d.path(), &discover(d.path(), &config).unwrap(), s.binding).unwrap();
    assert_ne!(new.digest, changed_ids.digest);
    assert_eq!(parse(&changed_ids).requirements[0].key, key("R2"));
}

#[test]
fn bold_prose_is_not_discarded_as_metadata() {
    let extra = "**Important** prose contains **: and MUST remain in the requirement text.";
    let text = MAIN.replace(
        "#### Scenario: Correct password",
        &format!("{extra}\n\n#### Scenario: Correct password"),
    );
    let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let p = parse(&s);
    assert!(build_graph(p.clone()).complete());
    assert!(p.requirements[0].text.contains(extra));
}

#[test]
fn indented_code_is_an_explicitly_unsupported_native_capability() {
    let text = MAIN.replace(
        "#### Scenario: Correct password",
        "    The application MUST enforce another constraint.\n\n#### Scenario: Correct password",
    );
    let (_, s) = native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
    let p = parse(&s);
    assert!(p.sources.iter().any(|s| s.status == Terminal::Unsupported));
    assert!(!build_graph(p).complete());
}

#[test]
fn direct_child_heading_depths_cannot_hide_unmapped_scenarios() {
    for (format, source) in [
        ("openspec/1.14.1-main", MAIN),
        ("openspec/1.14.1-added", ADDED),
    ] {
        for depth in 4..=6 {
            let extra = format!(
                "{} Scenario: Extra mandatory behavior\n- **THEN** the system rejects revoked credentials\n\n",
                "#".repeat(depth)
            );
            let text = source.replace(
                "#### Scenario: Correct password",
                &(extra + "#### Scenario: Correct password"),
            );
            let (_, s) = native_snapshot(format, text.as_bytes(), IDS.as_bytes());
            let p = parse(&s);
            assert!(
                !build_graph(p.clone()).complete(),
                "{format}, direct child H{depth}"
            );
            assert!(
                p.requirements.is_empty(),
                "partial native extraction must not escape"
            );
            if depth > 4 {
                assert!(p.sources.iter().any(|s| s.status == Terminal::Unsupported));
                let mut registry: serde_json::Value = serde_json::from_str(IDS).unwrap();
                registry["documents"][0]["requirements"][0]["scenarios"]
                    .as_array_mut()
                    .unwrap()
                    .push(serde_json::json!({"title":"Extra mandatory behavior","id":"A3"}));
                let (_, s) = native_snapshot(
                    format,
                    text.as_bytes(),
                    &serde_json::to_vec(&registry).unwrap(),
                );
                let mapped = parse(&s);
                assert!(!build_graph(mapped.clone()).complete());
                assert!(
                    mapped
                        .sources
                        .iter()
                        .any(|s| s.status == Terminal::Unsupported),
                    "mapping does not enable unsupported hierarchy"
                );
            }
        }
    }
}

#[test]
fn skipped_requirement_levels_cannot_hide_unmapped_requirements() {
    for (format, source) in [
        ("openspec/1.14.1-main", MAIN),
        ("openspec/1.14.1-added", ADDED),
    ] {
        for depth in 4..=5 {
            let extra = format!(
                "{} Requirement: Extra mandatory requirement\nThe application MUST enforce revoked credentials.\n\n{} Scenario: Revoked credential\n- **THEN** the system rejects access\n\n",
                "#".repeat(depth),
                "#".repeat(depth + 1)
            );
            let text = source.replace(
                "### Requirement: Password authentication",
                &(extra + "### Requirement: Password authentication"),
            );
            let (_, s) = native_snapshot(format, text.as_bytes(), IDS.as_bytes());
            let p = parse(&s);
            assert!(
                !build_graph(p.clone()).complete(),
                "{format}, requirement H{depth}"
            );
            assert!(p.sources.iter().any(|s| s.status == Terminal::Unsupported));
        }
    }
}

#[test]
fn fenced_or_quoted_headings_are_data_and_nested_scenario_headings_are_preserved() {
    for depth in 4..=6 {
        for wrapper in ["backtick", "tilde", "quote", "plain"] {
            let heading = format!(
                "{} Scenario: Extra mandatory behavior\n- **THEN** the system rejects revoked credentials\n",
                "#".repeat(depth)
            );
            let fragment = match wrapper {
                "backtick" => format!("```markdown\n{heading}```\n"),
                "tilde" => format!("~~~markdown\n{heading}~~~\n"),
                "quote" => heading.lines().map(|line| format!("> {line}\n")).collect(),
                _ => heading,
            };
            for nested in [false, true] {
                let anchor = if nested {
                    "#### Scenario: Incorrect password"
                } else {
                    "#### Scenario: Correct password"
                };
                let text = MAIN.replace(anchor, &format!("{fragment}\n{anchor}"));
                let (_, s) =
                    native_snapshot("openspec/1.14.1-main", text.as_bytes(), IDS.as_bytes());
                let p = parse(&s);
                if wrapper == "plain" && (!nested || depth == 4) {
                    assert!(
                        !build_graph(p.clone()).complete(),
                        "unmapped H{depth}, nested={nested}"
                    );
                    assert!(p.acceptances.is_empty());
                } else {
                    assert!(
                        build_graph(p.clone()).complete(),
                        "H{depth}, {wrapper}, nested={nested}: {:?}",
                        p.sources
                    );
                    assert_eq!(p.acceptances.len(), 2);
                    if nested {
                        assert!(p.acceptances[0].text.contains("Extra mandatory behavior"));
                    }
                }
            }
        }
    }
}
