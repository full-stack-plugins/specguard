use serde_json::Value;
use std::path::Path;

/// 分发原生规格只读检查和复验；要求显式来源，避免自动选择冲突事实源。
pub(crate) fn run(args: &[String]) -> Result<(Value, u8), String> {
    if args.len() == 4 && args[0] == "specs" && args[1] == "sources" && args[2] == "--project" {
        let project = Path::new(&args[3]);
        if !project.is_absolute() || !project.is_dir() {
            return Err("project must be an existing absolute directory".into());
        }
        return crate::spec_sources::check(&project.canonicalize().map_err(|e| e.to_string())?);
    }
    if args.len() < 6
        || args[0] != "specs"
        || args[2] != "--project"
        || args[4] != "--source"
        || !["openspec", "speckit"].contains(&args[5].as_str())
        || !((args[1] == "check" && args.len() == 6)
            || (args[1] == "verify" && args.len() == 8 && args[6] == "--baseline"))
    {
        return Err("usage: specguard specs check|verify --project ABSOLUTE_DIRECTORY --source openspec|speckit [--baseline ABSOLUTE_REPORT]".into());
    }
    let project = Path::new(&args[3]);
    if !project.is_absolute() || !project.is_dir() {
        return Err("project must be an existing absolute directory".into());
    }
    let baseline = args.get(7).map(Path::new);
    if baseline.is_some_and(|p| !p.is_absolute() || !p.is_file()) {
        return Err("baseline must be an existing absolute report file".into());
    }
    crate::specs_check::check(
        &project.canonicalize().map_err(|e| e.to_string())?,
        &args[5],
        baseline,
    )
}
