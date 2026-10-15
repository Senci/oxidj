//! OxiDJ CLI entry point.
//!
//! M0 (ADR-004): the CLI is the command-first caller into `oxidj-core`. It
//! invokes the same core domain handlers the GUI would, and never touches
//! persistence, audio, or vendor adapters directly.
//!
//! This binary is deliberately thin. Each user-facing verb maps to a domain
//! operation in `oxidj-core`; orchestration lives in core/library, not here.

use oxidj_core::TrackId;

/// Exit code for usage / argument-shape errors.
const USAGE: i32 = 2;
/// Exit code for input that is well-formed but violates a domain rule.
const DOMAIN: i32 = 1;
/// Success.
const OK: i32 = 0;

/// CLI dispatch, factored out of `main` so it is unit-testable without spawning
/// a process. `args[0]` is the program name. Returns a process exit code.
fn run(args: &[String]) -> i32 {
    let Some(command) = args.get(1).map(String::as_str) else {
        print_usage();
        return USAGE;
    };
    match command {
        "track-id" => match args.get(2) {
            Some(raw) => match raw.parse::<u64>() {
                Ok(n) => match TrackId::new(n) {
                    Ok(id) => {
                        println!("{id}");
                        OK
                    }
                    Err(oxidj_core::TrackIdError::Zero) => {
                        eprintln!("error: track id must be >= 1 (0 is reserved)");
                        DOMAIN
                    }
                },
                Err(_) => {
                    eprintln!("error: argument must be an unsigned integer");
                    USAGE
                }
            },
            None => {
                eprintln!("error: track-id requires a numeric argument");
                USAGE
            }
        },
        unknown => {
            eprintln!("error: unknown command: {unknown}");
            USAGE
        }
    }
}

fn print_usage() {
    eprintln!("usage: oxidj-cli <command>");
    eprintln!("commands:");
    eprintln!("  track-id <n>   Construct and echo a domain TrackId (n >= 1)");
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let code = u8::try_from(run(&args)).unwrap_or(1);
    std::process::ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(rest: &[&str]) -> Vec<String> {
        let mut v = vec!["oxidj-cli".to_string()];
        v.extend(rest.iter().map(|s| s.to_string()));
        v
    }

    #[test]
    fn no_command_is_a_usage_error() {
        assert_ne!(run(&argv(&[])), OK);
    }

    #[test]
    fn unknown_command_is_a_usage_error() {
        assert_ne!(run(&argv(&["bogus"])), OK);
    }

    #[test]
    fn zero_track_id_is_a_domain_error() {
        assert_eq!(run(&argv(&["track-id", "0"])), DOMAIN);
    }

    #[test]
    fn one_track_id_succeeds() {
        assert_eq!(run(&argv(&["track-id", "1"])), OK);
    }
}
