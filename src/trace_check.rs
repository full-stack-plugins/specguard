use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};

fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 2048)
        .ok_or_else(|| format!("bounded {key} string required"))
}
fn local_text(root: &Path, relative: &str, total: &mut u64) -> Result<String, String> {
    let p = Path::new(relative);
    if p.components().any(|c| !matches!(c, Component::Normal(_))) || relative.contains('\\') {
        return Err("project-relative normal path required".into());
    }
    let path = root.join(p);
    let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
    *total = total.saturating_add(size);
    if size > 1024 * 1024 || *total > 16 * 1024 * 1024 {
        return Err("mapped source byte budget exceeded".into());
    }
    // 共享有界输入先拒绝所有祖先链接，再读取映射内容。
    guard_evidence::hash_file(&path)?;
    let resolved = path.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(root)
        || fs::metadata(&resolved).map_err(|e| e.to_string())?.len() > 1024 * 1024
    {
        return Err("mapped source outside project or exceeds 1MiB".into());
    }
    fs::read_to_string(resolved).map_err(|e| e.to_string())
}

/// 校验原生需求声明映射与受信执行证据，返回报告和退出码；不证明业务语义。无 Java 对应。
pub(crate) fn check(
    root: &Path,
    manifest_path: &Path,
    evidence: Option<&Path>,
    key: Option<&Path>,
) -> Result<(Value, u8), String> {
    let snapshot_before = guard_evidence::snapshot_project(root)?;
    let manifest = guard_evidence::read_json(manifest_path)?;
    if manifest["schema_version"] != "specguard.trace/v1"
        || !matches!(manifest["source"].as_str(), Some("openspec" | "speckit"))
    {
        return Err("specguard.trace/v1 and explicit native source required".into());
    }
    let allowed = [
        "schema_version",
        "source",
        "execution_plan_digest",
        "mappings",
    ];
    if manifest
        .as_object()
        .is_none_or(|o| o.keys().any(|k| !allowed.contains(&k.as_str())))
    {
        return Err("unknown trace manifest field".into());
    }
    let plan = string(&manifest, "execution_plan_digest")?;
    if !plan.starts_with("sha256:")
        || plan.len() != 71
        || !plan[7..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("explicit execution plan digest required".into());
    }
    let (native, native_code) =
        crate::specs_check::check(root, manifest["source"].as_str().unwrap(), None)?;
    let mut findings = Vec::<String>::new();
    if native_code != 0 {
        findings.push("native specification structure or inventory is incomplete".into());
    }
    let requirements: BTreeSet<(String, String)> = native["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["operation"] == "defined")
        .map(|v| {
            (
                v["path"].as_str().unwrap().to_owned(),
                v["id"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let mappings = manifest["mappings"]
        .as_array()
        .filter(|v| v.len() <= 8192)
        .ok_or("bounded mappings array required")?;
    let mut covered = BTreeSet::new();
    let mut tests = BTreeSet::new();
    let mut count = 0usize;
    let mut total = 0u64;
    for mapping in mappings {
        let result = (|| -> Result<(), String> {
            let artifact = string(mapping, "artifact")?;
            let id = string(mapping, "requirement_id")?;
            let reference = (artifact.to_owned(), id.to_owned());
            if !requirements.contains(&reference) || !covered.insert(reference) {
                return Err("unknown or duplicate requirement artifact/id".into());
            }
            let code = mapping["code"]
                .as_array()
                .filter(|v| !v.is_empty() && v.len() <= 128)
                .ok_or("nonempty bounded code references required")?;
            for reference in code {
                count += 1;
                if count > 1024 {
                    return Err("mapped source read budget exceeded".into());
                }
                let marker = string(reference, "marker")?;
                if marker.trim().len() < 3
                    || !local_text(root, string(reference, "path")?, &mut total)?.contains(marker)
                {
                    return Err("mapped source marker missing or empty".into());
                }
            }
            let ids = mapping["tests"]
                .as_array()
                .filter(|v| !v.is_empty() && v.len() <= 8192)
                .ok_or("nonempty bounded test IDs required")?;
            for id in ids {
                tests.insert(
                    id.as_str()
                        .filter(|s| !s.trim().is_empty() && s.len() <= 2048)
                        .ok_or("empty test ID")?
                        .to_owned(),
                );
            }
            Ok(())
        })();
        if let Err(e) = result {
            findings.push(e);
        }
    }
    if requirements.is_empty() || covered != requirements {
        findings.push("every defined native requirement needs a mapping".into());
    }
    let mut verified = Value::Null;
    let mut execution_error = None;
    if let (Some(evidence), Some(key)) = (evidence, key) {
        match guard_evidence::read_json(evidence)
            .and_then(|e| guard_evidence::verify_execution(&e, key, root, plan, 86400))
        {
            Ok(payload) => {
                for test in &tests {
                    if !payload["required_tests"]
                        .as_array()
                        .is_some_and(|a| a.contains(&json!(test)))
                        || !payload["observed_tests"]
                            .as_array()
                            .is_some_and(|a| a.contains(&json!(test)))
                    {
                        findings.push(format!(
                            "mapped test missing from required/observed execution: {test}"
                        ));
                    }
                }
                verified = json!({"payload_digest":guard_evidence::digest_json(&payload)?,"plan_digest":plan});
            }
            Err(e) => execution_error = Some(e),
        }
    }
    if let Some(e) = execution_error {
        findings.push(format!("execution evidence rejected: {e}"));
    }
    let snapshot = guard_evidence::snapshot_project(root)?;
    if snapshot != snapshot_before {
        findings.push("candidate changed during trace validation".into());
    }
    let code = if !findings.is_empty() {
        2
    } else if verified.is_null() {
        3
    } else {
        0
    };
    Ok((
        json!({"schema_version":"specguard.trace-report/v1","project":root.to_string_lossy(),"status":match code{0=>"PASS",2=>"FAIL",_=>"INDETERMINATE"},"manifest_digest":guard_evidence::digest_json(&manifest)?,"snapshot":snapshot,"native":native,"required_tests":tests,"execution":verified,"findings":findings,"semantic_validation":"NOT_EVALUATED","approval_status":"NOT_EVALUATED","claim":"Declared source markers and test evidence linkage; not proof of semantic implementation"}),
        code,
    ))
}
