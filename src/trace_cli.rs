use serde_json::Value;
use std::{collections::BTreeMap, path::Path};
/// 分发追踪检查或完整重算复验；返回 JSON 与退出码，不写项目文件。无 Java 对应。
pub(crate) fn run(args: &[String]) -> Result<(Value, u8), String> {
    if args.len() < 6
        || !matches!(args[1].as_str(), "check" | "verify")
        || (args.len() - 2) % 2 != 0
    {
        return Err("usage: trace check|verify --project ABS --manifest ABS [--evidence ABS --public-key ABS] [--report ABS]".into());
    }
    let mut flags = BTreeMap::new();
    for pair in args[2..].chunks(2) {
        if ![
            "--project",
            "--manifest",
            "--evidence",
            "--public-key",
            "--report",
        ]
        .contains(&pair[0].as_str())
            || flags.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown or duplicate trace option".into());
        }
    }
    if flags.contains_key("--evidence") != flags.contains_key("--public-key")
        || (args[1] == "verify") != flags.contains_key("--report")
    {
        return Err("evidence/key must be paired; verify requires report".into());
    }
    let project = Path::new(*flags.get("--project").ok_or("project required")?);
    if !project.is_absolute() || !project.is_dir() {
        return Err("absolute existing project required".into());
    }
    // 快照 API 拒绝根及祖先链接。
    guard_evidence::snapshot_project(project)?;
    let root = project.canonicalize().map_err(|e| e.to_string())?;
    let manifest = Path::new(*flags.get("--manifest").ok_or("manifest required")?);
    let (mut report, mut code) = crate::trace_check::check(
        &root,
        manifest,
        flags.get("--evidence").map(Path::new),
        flags.get("--public-key").map(Path::new),
    )?;
    if let Some(previous) = flags.get("--report") {
        let previous = guard_evidence::read_json(Path::new(previous))?;
        if previous != report {
            report["findings"]
                .as_array_mut()
                .unwrap()
                .push(Value::String(
                    "previous trace report differs from full current recomputation".into(),
                ));
            report["status"] = Value::String("FAIL".into());
            code = 2;
        }
    }
    Ok((report, code))
}
