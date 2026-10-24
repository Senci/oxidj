//! OxiDJ CLI entry point.
//!
//! The CLI calls shared core and library handlers; persistence stays behind
//! `oxidj-library`, and audio, UI, and vendor adapters stay out of this crate.
//!
//! This binary is deliberately thin. Each user-facing verb maps to a domain
//! operation in `oxidj-core` or `oxidj-library`; orchestration lives there.

use oxidj_core::TrackId;
use oxidj_library::Catalog;

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
        print_usage(false);
        return USAGE;
    };
    match command {
        "--help" | "-h" => {
            print_usage(true);
            OK
        }
        "--version" | "-V" => {
            println!("oxidj-cli {}", env!("CARGO_PKG_VERSION"));
            OK
        }
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
                    Err(oxidj_core::TrackIdError::Parse) => {
                        eprintln!("error: argument must be an unsigned integer");
                        USAGE
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
        "catalog" => run_catalog(&args[2..]),
        unknown => {
            eprintln!("error: unknown command: {unknown}");
            USAGE
        }
    }
}

fn run_catalog(args: &[String]) -> i32 {
    let Some(action) = args.first().map(String::as_str) else {
        eprintln!("error: catalog requires init, list, backup, or restore");
        return USAGE;
    };
    if action == "import" {
        let Some(dir_index) = args.iter().position(|arg| arg == "--dir") else {
            eprintln!("error: catalog import requires --dir PATH");
            return USAGE;
        };
        let Some(db_index) = args.iter().position(|arg| arg == "--db") else {
            eprintln!("error: catalog import requires --db PATH");
            return USAGE;
        };
        if args.len() != 5 || dir_index + 1 >= args.len() || db_index + 1 >= args.len() {
            eprintln!("error: invalid catalog import arguments");
            return USAGE;
        }
        let directory = &args[dir_index + 1];
        let db = &args[db_index + 1];
        return match Catalog::open(db)
            .and_then(|mut catalog| catalog.import_audio_directory(directory))
        {
            Ok(report) => {
                for outcome in report.outcomes {
                    println!("{:?}\t{}", outcome.status, outcome.path);
                }
                if report.complete {
                    OK
                } else {
                    eprintln!("warning: scan incomplete; missing status unchanged");
                    DOMAIN
                }
            }
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        };
    }
    if action == "backup" || action == "restore" {
        let (source_flag, destination_flag) = if action == "backup" {
            ("--db", "--to")
        } else {
            ("--from", "--db")
        };
        let Some(source_index) = args.iter().position(|arg| arg == source_flag) else {
            eprintln!("error: catalog {action} requires {source_flag} PATH");
            return USAGE;
        };
        let Some(destination_index) = args.iter().position(|arg| arg == destination_flag) else {
            eprintln!("error: catalog {action} requires {destination_flag} PATH");
            return USAGE;
        };
        if args.len() != 5 || source_index + 1 >= args.len() || destination_index + 1 >= args.len()
        {
            eprintln!("error: invalid catalog {action} arguments");
            return USAGE;
        }
        let source = &args[source_index + 1];
        let destination = &args[destination_index + 1];
        let result = if action == "backup" {
            Catalog::backup_from(source, destination)
        } else {
            Catalog::restore_from(source, destination)
        };
        return match result {
            Ok(()) => OK,
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        };
    }
    let Some(db_index) = args.iter().position(|arg| arg == "--db") else {
        eprintln!("error: catalog {action} requires --db PATH");
        return USAGE;
    };
    let Some(path) = args.get(db_index + 1) else {
        eprintln!("error: --db requires a path");
        return USAGE;
    };
    if args.len() != db_index + 2 {
        eprintln!("error: unexpected catalog argument");
        return USAGE;
    }

    match action {
        "init" => match Catalog::open(path) {
            Ok(_) => OK,
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        },
        "list" => match Catalog::open(path).and_then(|catalog| catalog.list_tracks()) {
            Ok(tracks) => {
                println!("TrackId\tPath\tSHA-256\tBytes");
                for track in tracks {
                    let digest = track
                        .sha256
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>();
                    println!(
                        "{}\t{}\t{}\t{}",
                        track.id, track.path, digest, track.byte_length
                    );
                }
                OK
            }
            Err(error) => {
                eprintln!("error: {error}");
                DOMAIN
            }
        },
        _ => {
            eprintln!("error: unknown catalog action: {action}");
            USAGE
        }
    }
}

fn print_usage(to_stdout: bool) {
    let output = if to_stdout {
        &mut std::io::stdout().lock() as &mut dyn std::io::Write
    } else {
        &mut std::io::stderr().lock() as &mut dyn std::io::Write
    };
    let _ = writeln!(output, "usage: oxidj-cli <command>");
    let _ = writeln!(output, "commands:");
    let _ = writeln!(
        output,
        "  track-id <n>   Construct and echo a domain TrackId (n >= 1)"
    );
    let _ = writeln!(
        output,
        "  catalog init --db <path>   Initialize local catalog"
    );
    let _ = writeln!(
        output,
        "  catalog list --db <path>   List local catalog tracks"
    );
    let _ = writeln!(
        output,
        "  catalog import --dir <directory> --db <path>   Import WAV/FLAC/MP3 references recursively"
    );
    let _ = writeln!(
        output,
        "  catalog backup --db <source> --to <new-backup>   Create verified backup"
    );
    let _ = writeln!(
        output,
        "  catalog restore --from <backup> --db <new-destination>   Restore backup"
    );
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
