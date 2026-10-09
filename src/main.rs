use specguard::{
    model::Identity,
    source::{CandidateBinding, SourcePolicy, discover, freeze},
    trace,
};
use std::{collections::BTreeSet, io::Read, path::Path};
fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("configuration must be a regular file".into());
    }
    let mut bytes = vec![];
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 1_048_576 {
        return Err("configuration byte limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
fn run() -> Result<i32, String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 || !matches!(args[1].as_str(), "trace-check" | "trace-export") {
        return Err(
            "usage: specguard trace-check|trace-export ROOT POLICY.json BINDING.json REQUIRED.json"
                .into(),
        );
    }
    let policy: SourcePolicy = read(&args[3])?;
    let binding: CandidateBinding = read(&args[4])?;
    let required: BTreeSet<Identity> = read(&args[5])?;
    let root = Path::new(&args[2]);
    let inventory = discover(root, &policy)?;
    let snapshot = freeze(root, &inventory, binding)?;
    let artifact = trace::scan(&snapshot, &required)?;
    let bytes = trace::export(&artifact)?;
    use std::io::Write;
    std::io::stdout()
        .write_all(&bytes)
        .map_err(|e| e.to_string())?;
    Ok(
        if artifact.graph.complete() && artifact.findings.is_empty() {
            0
        } else {
            2
        },
    )
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("specguard: {error}");
            std::process::exit(4);
        }
    }
}
