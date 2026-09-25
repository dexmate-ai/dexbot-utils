use clap::{Parser, Subcommand};
use dexbot_model::{
    available_profiles, migrate_profile, try_profile_for_robot_name, ResolvedRobotConfig,
    RobotConfig, UrdfModel, SOURCE_SCHEMA_VERSION,
};
use serde_json::Value;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// List the built-in robot profiles.
    List,
    /// Print the normalized resolved configuration of a built-in profile.
    Show { profile: String },
    /// Validate a profile file and print a verdict summary.
    Validate {
        path: PathBuf,
        /// Print the full normalized resolved configuration instead.
        #[arg(long)]
        json: bool,
    },
    /// Field-level diff between two resolved configurations. Each side is a
    /// built-in profile name or a profile file path.
    Diff {
        left: String,
        right: String,
        /// Exit with status 1 when the configurations differ (like
        /// `git diff --exit-code`); by default a difference exits 0.
        #[arg(long)]
        exit_code: bool,
    },
    /// Migrate a profile file between schema versions (defaults to the
    /// current schema version; same-version migration is the identity).
    /// The migrated profile must resolve and validate, or nothing is printed.
    Migrate {
        path: PathBuf,
        /// Target schema version.
        #[arg(long)]
        to: Option<u32>,
    },
    /// Summarize a URDF file: robot name, link/joint counts, movable joints.
    Urdf { path: PathBuf },
    /// Print the built-in profile serving a ROBOT_NAME-style robot
    /// identifier (e.g. dm/vg0123456789-1u -> vega_1u). Fails on a name
    /// that selects no built-in profile.
    ProfileFor { name: String },
}

#[derive(Parser)]
#[command(
    name = "dexbot",
    version,
    about = "Inspect and validate Dexmate robot models"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Command failure that carries no message of its own: `diff --exit-code`
/// already printed the differences, which are its report.
#[derive(Debug)]
struct Differences;

impl std::fmt::Display for Differences {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the configurations differ")
    }
}

impl Error for Differences {}

/// Runs the CLI with process semantics: `args` excludes the program name and
/// the return value is the exit code — 0 on success (including `--help` and
/// `--version`), 2 on a usage error (clap's convention), 1 on a command
/// failure (reported on stderr).
pub fn run(args: Vec<String>) -> i32 {
    let cli = match Cli::try_parse_from(std::iter::once("dexbot".to_owned()).chain(args)) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return error.exit_code();
        }
    };
    match execute(cli.command, &mut std::io::stdout()) {
        Ok(()) => 0,
        Err(error) if error.is::<Differences>() => 1,
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}

pub fn execute(command: Command, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    match command {
        Command::List => {
            for profile in available_profiles() {
                writeln!(output, "{profile}")?;
            }
        }
        Command::Show { profile } => write!(
            output,
            "{}",
            RobotConfig::from_profile(&profile)?
                .resolve()?
                .normalized_json()?
        )?,
        Command::Validate { path, json } => {
            let resolved = RobotConfig::from_file(&path)?.resolve()?;
            if json {
                write!(output, "{}", resolved.normalized_json()?)?;
            } else {
                write_verdict(output, &path, &resolved)?;
            }
        }
        Command::Diff {
            left,
            right,
            exit_code,
        } => {
            let left_config = resolve_source(&left)?;
            let right_config = resolve_source(&right)?;
            let mut lines = Vec::new();
            diff_values(
                "",
                &serde_json::to_value(&left_config)?,
                &serde_json::to_value(&right_config)?,
                &mut lines,
            );
            if lines.is_empty() {
                writeln!(output, "identical")?;
            } else {
                for line in lines {
                    writeln!(output, "{line}")?;
                }
                if exit_code {
                    return Err(Box::new(Differences));
                }
            }
        }
        Command::Migrate { path, to } => {
            let source = std::fs::read_to_string(&path)?;
            let migrated = migrate_profile(&source, to.unwrap_or(SOURCE_SCHEMA_VERSION))?;
            // A migration that yields a profile this build rejects has not
            // migrated anything. Fragments resolve relative to the input.
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("migrated");
            let directory = path.parent().unwrap_or(Path::new("."));
            RobotConfig::from_yaml_in(name, &migrated, directory)?.resolve()?;
            write!(output, "{migrated}")?;
        }
        Command::Urdf { path } => {
            let model = UrdfModel::from_file(path)?;
            writeln!(
                output,
                "robot={} links={} joints={}",
                model.robot_name,
                model.links.len(),
                model.joints.len()
            )?;
            for joint in model.movable_joint_names() {
                writeln!(output, "{joint}")?;
            }
        }
        Command::ProfileFor { name } => writeln!(output, "{}", try_profile_for_robot_name(&name)?)?,
    }
    Ok(())
}

fn write_verdict(
    output: &mut impl Write,
    path: &Path,
    resolved: &ResolvedRobotConfig,
) -> std::io::Result<()> {
    let enabled = |objects: &std::collections::BTreeMap<String, dexbot_model::ComponentConfig>| {
        objects.values().filter(|object| object.enabled).count()
    };
    writeln!(
        output,
        "OK: {} resolves as profile {:?} (model {}, schema v{}, resolved v{})",
        path.display(),
        resolved.profile_name,
        resolved.robot.model,
        resolved.schema_version,
        resolved.resolved_config_version,
    )?;
    writeln!(
        output,
        "components: {} enabled / {} total; sensors: {} enabled / {} total",
        enabled(&resolved.components),
        resolved.components.len(),
        enabled(&resolved.sensors),
        resolved.sensors.len(),
    )?;
    writeln!(output, "content_hash: {}", resolved.content_hash)
}

/// Resolves a diff operand: an existing file path wins, otherwise the name
/// is looked up as a built-in profile.
fn resolve_source(source: &str) -> dexbot_model::Result<ResolvedRobotConfig> {
    if Path::new(source).is_file() {
        RobotConfig::from_file(source)?.resolve()
    } else {
        RobotConfig::from_profile(source)?.resolve()
    }
}

/// Derived and diagnostic fields excluded from the field-level diff.
const DIFF_EXCLUDED_ROOTS: &[&str] = &["content_hash", "provenance"];

fn diff_values(path: &str, left: &Value, right: &Value, lines: &mut Vec<String>) {
    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            let keys: std::collections::BTreeSet<&String> =
                left_map.keys().chain(right_map.keys()).collect();
            for key in keys {
                if path.is_empty() && DIFF_EXCLUDED_ROOTS.contains(&key.as_str()) {
                    continue;
                }
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (left_map.get(key), right_map.get(key)) {
                    (Some(left_value), Some(right_value)) => {
                        diff_values(&child, left_value, right_value, lines)
                    }
                    (Some(left_value), None) => {
                        lines.push(format!("{child}: {} -> <absent>", compact(left_value)))
                    }
                    (None, Some(right_value)) => {
                        lines.push(format!("{child}: <absent> -> {}", compact(right_value)))
                    }
                    (None, None) => unreachable!("key from one of the maps"),
                }
            }
        }
        (left, right) if left != right => {
            lines.push(format!("{path}: {} -> {}", compact(left), compact(right)));
        }
        _ => {}
    }
}

fn compact(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 120 {
        let truncated: String = text.chars().take(119).collect();
        format!("{truncated}…")
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn output(command: Command) -> String {
        let mut bytes = Vec::new();
        execute(command, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn list_and_show_work() {
        assert!(output(Command::List).contains("vega_1\n"));
        let shown = output(Command::Show {
            profile: "vega_1".into(),
        });
        assert!(shown.contains("\"profile_name\": \"vega_1\""));
    }

    #[test]
    fn diff_reports_field_level_changes_between_profiles() {
        assert_eq!(
            output(Command::Diff {
                left: "vega_1".into(),
                right: "vega_1".into(),
                exit_code: false,
            }),
            "identical\n"
        );
        let changed = output(Command::Diff {
            left: "vega_1".into(),
            right: "vega_1u".into(),
            exit_code: false,
        });
        // Removed components and scalar changes appear as path: old -> new.
        assert!(changed.contains("robot.model: \"vega_1\" -> \"vega_1u\""));
        assert!(changed.contains("components.torso:"));
        assert!(changed.contains("-> <absent>"));
        // Derived and diagnostic fields stay out of the diff.
        assert!(!changed.contains("content_hash"));
        assert!(!changed.contains("provenance"));
        // Hands variants add components.
        let added = output(Command::Diff {
            left: "vega_1".into(),
            right: "vega_1_f5d6".into(),
            exit_code: false,
        });
        assert!(added.contains("components.left_hand: <absent> ->"));
    }

    #[test]
    fn diff_exit_code_reports_differences_through_the_exit_status() {
        // Default: a difference is output, not a failure.
        assert_eq!(run_args(&["diff", "vega_1", "vega_1u"]), 0);
        // --exit-code: 1 on difference, 0 when identical, and the diff is
        // still written before the status is decided.
        assert_eq!(run_args(&["diff", "--exit-code", "vega_1", "vega_1u"]), 1);
        assert_eq!(run_args(&["diff", "--exit-code", "vega_1", "vega_1"]), 0);
        let mut sink = Vec::new();
        let error = execute(
            Command::Diff {
                left: "vega_1".into(),
                right: "vega_1u".into(),
                exit_code: true,
            },
            &mut sink,
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "the configurations differ");
        assert!(String::from_utf8(sink).unwrap().contains("robot.model"));
    }

    #[test]
    fn validate_prints_verdict_summary_or_full_json() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().to_path_buf();
        fs::create_dir_all(&directory).unwrap();
        let profile = directory.join("robot.yaml");
        fs::write(
            &profile,
            "schema_version: 1\nrobot:\n  model: test\n  profile_version: 1.0.0\n  asset_version: test\ncomponents: {}\n",
        )
        .unwrap();
        let verdict = output(Command::Validate {
            path: profile.clone(),
            json: false,
        });
        assert!(verdict.starts_with("OK: "));
        assert!(verdict.contains("model test"));
        assert!(verdict.contains("components: 0 enabled / 0 total"));
        assert!(verdict.contains("content_hash: "));
        let full = output(Command::Validate {
            path: profile.clone(),
            json: true,
        });
        assert!(full.contains("\"model\": \"test\""));
    }

    #[test]
    fn migrate_is_identity_for_current_schema_and_errors_otherwise() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().to_path_buf();
        fs::create_dir_all(&directory).unwrap();
        let profile = directory.join("robot.yaml");
        let source = "schema_version: 1\nrobot:\n  model: test\n# note\n";
        fs::write(&profile, source).unwrap();
        assert_eq!(
            output(Command::Migrate {
                path: profile.clone(),
                to: None,
            }),
            source
        );
        let mut sink = Vec::new();
        assert!(execute(
            Command::Migrate {
                path: profile.clone(),
                to: Some(2),
            },
            &mut sink,
        )
        .is_err());
        // The output must itself be a valid profile: a same-version
        // "migration" of a profile this build rejects prints nothing.
        fs::write(
            &profile,
            "schema_version: 1\nrobot:\n  model: test\nruntime:\n  state_idle_timeout_ms: 0\n",
        )
        .unwrap();
        let mut sink = Vec::new();
        let error = execute(
            Command::Migrate {
                path: profile.clone(),
                to: None,
            },
            &mut sink,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("state_idle_timeout_ms"),
            "{error}"
        );
        assert!(sink.is_empty());
        // Fragments next to the input are found during that check.
        fs::write(directory.join("base.yaml"), "robot:\n  model: test\n").unwrap();
        let extending = "schema_version: 1\nextends:\n- base.yaml\n";
        fs::write(&profile, extending).unwrap();
        assert_eq!(
            output(Command::Migrate {
                path: profile.clone(),
                to: None,
            }),
            extending
        );
    }

    #[test]
    fn urdf_command_reads_files() {
        let temporary = tempfile::tempdir().unwrap();
        let directory = temporary.path().to_path_buf();
        fs::create_dir_all(&directory).unwrap();
        let urdf = directory.join("robot.urdf");
        fs::write(
            &urdf,
            r#"<robot name="cli"><link name="base"/><link name="tip"/><joint name="axis" type="revolute"><parent link="base"/><child link="tip"/></joint></robot>"#,
        )
        .unwrap();
        let result = output(Command::Urdf { path: urdf.clone() });
        assert!(result.contains("robot=cli links=2 joints=1"));
        assert!(result.contains("axis\n"));
    }

    #[test]
    fn invalid_inputs_return_errors() {
        let mut output = Vec::new();
        assert!(execute(
            Command::Show {
                profile: "missing".into(),
            },
            &mut output,
        )
        .is_err());
        assert!(execute(
            Command::Validate {
                path: PathBuf::from("missing-file.yaml"),
                json: false,
            },
            &mut output,
        )
        .is_err());
        assert!(execute(
            Command::Diff {
                left: "missing".into(),
                right: "vega_1".into(),
                exit_code: false,
            },
            &mut output,
        )
        .is_err());
    }

    #[test]
    fn profile_for_maps_robot_names_to_profiles() {
        assert_eq!(
            output(Command::ProfileFor {
                name: "dm/vg0123456789-1u".into(),
            }),
            "vega_1u\n"
        );
        assert_eq!(
            output(Command::ProfileFor {
                name: "vega_1p_f5d6".into(),
            }),
            "vega_1p_f5d6\n"
        );
        // Unrecognised names fail instead of silently selecting vega_1.
        for name in ["unknown", "vega_1p_grippr", "dm/vg2-000123"] {
            let mut sink = Vec::new();
            let error = execute(Command::ProfileFor { name: name.into() }, &mut sink).unwrap_err();
            assert!(error.to_string().contains(name), "{error}");
            assert!(sink.is_empty());
        }
    }

    fn run_args(args: &[&str]) -> i32 {
        run(args.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn run_preserves_process_exit_code_semantics() {
        // 0: success, including clap's --help/--version paths.
        assert_eq!(run_args(&["list"]), 0);
        assert_eq!(run_args(&["profile-for", "dm/vg0123456789-1u"]), 0);
        assert_eq!(run_args(&["--help"]), 0);
        assert_eq!(run_args(&["--version"]), 0);
        // 2: usage errors (unknown subcommand, missing arguments).
        assert_eq!(run_args(&[]), 2);
        assert_eq!(run_args(&["not-a-command"]), 2);
        // 1: command failures.
        assert_eq!(run_args(&["show", "missing"]), 1);
        assert_eq!(run_args(&["profile-for", "dm/xyz-1u"]), 1);
    }
}

#[cfg(test)]
mod source_tests {
    #[test]
    fn an_existing_file_is_resolved_instead_of_a_builtin_profile() {
        let temporary = tempfile::tempdir().unwrap();
        let path = temporary.path().join("vega_1");
        std::fs::write(&path, "schema_version: 1\nrobot: {model: custom}\n").unwrap();
        assert_eq!(
            super::resolve_source(path.to_str().unwrap())
                .unwrap()
                .robot
                .model,
            "custom"
        );
        assert_eq!(
            super::resolve_source("vega_1").unwrap().robot.model,
            "vega_1"
        );
        std::fs::write(&path, "not: a_valid_profile").unwrap();
        assert!(super::resolve_source(path.to_str().unwrap()).is_err());
    }
}
