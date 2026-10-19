// The default binary must retain its process interface after consolidation.
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dexbot"))
        .args(args)
        .output()
        .expect("run packaged dexbot binary")
}

#[test]
fn help_version_and_profile_commands_work() {
    for args in [&["--help"][..], &["--version"][..]] {
        let output = run(args);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("dexbot"));
    }
    let version = run(&["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        format!("dexbot {}", env!("CARGO_PKG_VERSION"))
    );
    let list = run(&["list"]);
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout)
        .lines()
        .any(|p| p == "vega_1p"));
    let shown = run(&["show", "vega_1p"]);
    assert!(shown.status.success());
    let config: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(config["profile_name"], "vega_1p");
}

#[test]
fn usage_and_command_errors_keep_distinct_exit_codes() {
    let invalid = run(&["not-a-command"]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(!invalid.stderr.is_empty());
    let missing = run(&["show", "not-a-profile"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(!missing.stderr.is_empty());
    let same = run(&["diff", "vega_1", "vega_1", "--exit-code"]);
    assert!(same.status.success());
    let different = run(&["diff", "vega_1", "vega_1p", "--exit-code"]);
    assert_eq!(different.status.code(), Some(1));
}
