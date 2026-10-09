use std::{io::Read, path::Path};
fn start() -> Result<(), ()> {
    let args: Vec<_> = std::env::args_os().take(6).collect();
    if args.len() != 5 || args[1] != "--local-readonly-v1" {
        return Err(());
    }
    let config_path = Path::new(&args[3]);
    let meta = std::fs::symlink_metadata(config_path).map_err(|_| ())?;
    if !meta.is_file() || meta.len() > 256 * 1024 {
        return Err(());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(config_path).map_err(|_| ())?;
    if !file.metadata().map_err(|_| ())?.is_file() {
        return Err(());
    }
    let mut bytes = Vec::new();
    file.take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > 256 * 1024
        || Some(specguard::model::digest(&bytes).as_str()) != args[4].to_str()
    {
        return Err(());
    }
    let config = serde_json::from_slice(&bytes).map_err(|_| ())?;
    let session =
        specguard::mcp::ReadOnlySession::freeze(Path::new(&args[2]), config).map_err(|_| ())?;
    specguard::mcp::serve(
        session,
        std::io::stdin().lock(),
        std::io::stdout().lock(),
        &specguard::integration::runtime::CancellationToken::new(),
    )
    .map_err(|_| ())
}
fn main() -> std::process::ExitCode {
    match start() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(()) => {
            eprintln!("specguard MCP: local profile or transport unavailable");
            std::process::ExitCode::from(4)
        }
    }
}
