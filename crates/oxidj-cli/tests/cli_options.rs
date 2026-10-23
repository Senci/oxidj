use oxidj_core as _;
use oxidj_library as _;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_oxidj-cli"))
        .args(args)
        .output()
        .expect("failed to run oxidj-cli")
}

#[test]
fn help_options_print_usage_to_stdout() {
    for option in ["--help", "-h"] {
        let output = run(&[option]);
        assert!(output.status.success(), "{option}: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("usage: oxidj-cli <command>"));
        assert!(stdout.contains("track-id"));
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn version_options_print_binary_name_and_package_version() {
    for option in ["--version", "-V"] {
        let output = run(&[option]);
        assert!(output.status.success(), "{option}: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("oxidj-cli"));
        assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
        assert!(output.stderr.is_empty());
    }
}
