mod agent_integration;
mod agents_check;
mod agents_cli;
mod agents_policy;
mod markdown_document;
mod specs_check;
mod specs_cli;
mod text_requirement;
mod trace_check;
mod trace_cli;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = guard_install::dispatch(&agent_integration::tool(), &args) {
        return code;
    }
    match args.first().map(String::as_str) {
        Some("agents" | "specs" | "trace") => match if args[0] == "trace" {
            trace_cli::run(&args)
        } else if args[0] == "agents" {
            agents_cli::run(&args)
        } else {
            specs_cli::run(&args)
        } {
            Ok((result, code)) => {
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
                ExitCode::from(code)
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(4)
            }
        },
        Some("--version") if args.len() == 1 => {
            println!("specguard {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help" | "-h") | None => {
            println!(
                "specguard: trace check|verify --project ABS --manifest ABS [--evidence ABS --public-key ABS] [--report ABS]; agents check|template --project ABSOLUTE_DIRECTORY; specs check|verify --project ABSOLUTE_DIRECTORY --source openspec|speckit [--baseline ABSOLUTE_REPORT]; install --agents codex,claude,gemini|none; uninstall; context; spec context; instructions render|sync; doctor --integration; serve --mcp\n只读结构检查不证明智能体执行、业务语义或审批通过。退出码 0=结构合规，2=违规/漂移，3=分析不完整，4=参数/基线错误。"
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("unsupported command; use --help for implemented capabilities");
            ExitCode::from(4)
        }
    }
}
