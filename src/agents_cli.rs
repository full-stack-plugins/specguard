use serde_json::Value;
use std::path::Path;

/// 分发只读文档命令；参数必须为 agents check/template --project 绝对目录。
pub(crate) fn run(args: &[String]) -> Result<(Value, u8), String> {
    if args.len() != 4
        || args[0] != "agents"
        || args[2] != "--project"
        || !["check", "template"].contains(&args[1].as_str())
    {
        return Err("usage: specguard agents check|template --project ABSOLUTE_DIRECTORY".into());
    }
    let path = Path::new(&args[3]);
    if !path.is_absolute() || !path.is_dir() {
        return Err("project must be an existing absolute directory".into());
    }
    let root = path.canonicalize().map_err(|e| e.to_string())?;
    if args[1] == "template" {
        Ok((crate::agents_check::template(), 0))
    } else {
        Ok(crate::agents_check::check(&root))
    }
}
