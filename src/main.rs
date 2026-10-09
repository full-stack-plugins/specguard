fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match specguard::cli::run(&args) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::json!({"code":"cli.error","message":error})
            );
            std::process::exit(4);
        }
    }
}
