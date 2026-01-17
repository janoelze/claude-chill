use claude_chill::proxy::{Proxy, ProxyConfig};
use std::env;
use std::process::ExitCode;
use std::str::FromStr;

fn parse_env_var<T: FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: claude-chill <command> [args...]");
        eprintln!();
        eprintln!("PTY proxy that reduces terminal flicker by truncating synchronized output.");
        eprintln!();
        eprintln!("Environment variables:");
        eprintln!("  CHILL_MAX_LINES    Max lines per sync block (default: 100)");
        eprintln!("  CHILL_HISTORY      Max history lines for lookback (default: 100000)");
        eprintln!();
        eprintln!("Lookback mode: Press Ctrl+Shift+PgUp to view full history");
        return ExitCode::from(1);
    }

    let command = &args[1];
    let cmd_args: Vec<&str> = args[2..].iter().map(|s| s.as_str()).collect();

    let config = ProxyConfig {
        max_output_lines: parse_env_var("CHILL_MAX_LINES", 100),
        max_history_lines: parse_env_var("CHILL_HISTORY", 100_000),
        ..Default::default()
    };

    match Proxy::spawn(command, &cmd_args, config) {
        Ok(mut proxy) => match proxy.run() {
            Ok(exit_code) => ExitCode::from(exit_code as u8),
            Err(e) => {
                eprintln!("Proxy error: {}", e);
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("Failed to start proxy: {}", e);
            ExitCode::from(1)
        }
    }
}
