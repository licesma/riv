//! The riv CLI: a thin dispatch layer over `riv_checker`.
//!
//! Verbs:
//! ```text
//! riv check              # validate all rivers under the current directory
//! riv <river> check   # validate one river
//! riv <river>         # execute one river (implies check; --no-check)
//! riv <river> run     # same, explicit form
//! ```
//!
//! `<river>` is a YAML path or a directory; a directory means its
//! `river.yaml`. Verbs win: a directory named `check` needs `riv check/`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use riv_checker::render::{OutputFormat, render_to_string};
use riv_checker::{CheckResult, check_all, check_river, resolve_run_steps};

#[derive(Parser)]
#[command(
    name = "riv",
    version,
    about = "Typed data rivers for Python",
    after_help = "`riv <river.yaml>` runs one river; `riv <dir>` runs the directory's river.yaml; `riv <river> check` validates it."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate all rivers under the current directory.
    Check {
        /// Output format: full, concise, json, or github.
        #[arg(long, default_value = "full")]
        output_format: String,
    },
    /// `riv <river.yaml> [check|run]` (bare form runs).
    #[command(external_subcommand)]
    River(Vec<String>),
}

/// Arguments after `riv <river.yaml>`, parsed with clap for real
/// `--help`/error behavior.
#[derive(Parser)]
#[command(name = "riv <river.yaml>", disable_version_flag = true)]
struct RiverCli {
    /// Skip the pre-run check (escape hatch).
    #[arg(long)]
    no_check: bool,
    /// Defaults to `run` when omitted.
    #[command(subcommand)]
    verb: Option<Verb>,
}

#[derive(Subcommand)]
enum Verb {
    /// Validate this river.
    Check {
        /// Output format: full, concise, json, or github.
        #[arg(long, default_value = "full")]
        output_format: String,
    },
    /// Execute this river. Refuses to start unless it type-checks.
    Run {
        /// Skip the pre-run check (escape hatch).
        #[arg(long)]
        no_check: bool,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Check { output_format } => {
            let Some(format) = parse_format(&output_format) else {
                return ExitCode::from(2);
            };
            report(&check_all(Path::new(".")), format)
        }
        Command::River(args) => river_command(&args),
    }
}

fn river_command(args: &[String]) -> ExitCode {
    let river = match resolve_river(&args[0]) {
        Ok(river) => river,
        Err(exit) => return exit,
    };
    let parsed = match RiverCli::try_parse_from(
        std::iter::once("riv <river.yaml>".to_string()).chain(args[1..].iter().cloned()),
    ) {
        Ok(parsed) => parsed,
        Err(err) => {
            let _ = err.print();
            return ExitCode::from(2);
        }
    };
    match parsed.verb {
        Some(Verb::Check { output_format }) => {
            let Some(format) = parse_format(&output_format) else {
                return ExitCode::from(2);
            };
            report(&check_river(&river), format)
        }
        Some(Verb::Run { no_check }) => run(&river, no_check || parsed.no_check),
        None => run(&river, parsed.no_check),
    }
}

/// Resolve the river argument: a YAML path is a river; a directory means
/// its `river.yaml`.
fn resolve_river(arg: &str) -> Result<PathBuf, ExitCode> {
    let path = PathBuf::from(arg);
    if matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yaml" | "yml")
    ) {
        return Ok(path);
    }
    if path.is_dir() {
        let entry = path.join("river.yaml");
        if entry.is_file() {
            return Ok(entry);
        }
        let mut found: Vec<String> = std::fs::read_dir(&path)
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        matches!(
                            Path::new(&name).extension().and_then(|x| x.to_str()),
                            Some("yaml" | "yml")
                        )
                        .then_some(name)
                    })
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        if found.is_empty() {
            eprintln!("error: no river.yaml in `{arg}`");
        } else {
            eprintln!(
                "error: no river.yaml in `{arg}`; found: {}",
                found.join(", ")
            );
        }
        return Err(ExitCode::from(2));
    }
    let mut cmd = <Cli as clap::CommandFactory>::command();
    eprintln!(
        "{}",
        cmd.error(
            clap::error::ErrorKind::InvalidSubcommand,
            format!("unrecognized subcommand, river, or directory `{arg}`"),
        )
        .render()
    );
    Err(ExitCode::from(2))
}

fn parse_format(raw: &str) -> Option<OutputFormat> {
    match raw.parse() {
        Ok(format) => Some(format),
        Err(err) => {
            eprintln!("error: {err}");
            None
        }
    }
}

/// Print diagnostics and a summary; exit 1 when errors were found.
fn report(result: &CheckResult, format: OutputFormat) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    let _ = write!(stdout, "{}", render_to_string(&result.diagnostics, format));
    let (errors, warnings) = (result.error_count(), result.warning_count());
    if matches!(format, OutputFormat::Full | OutputFormat::Concise) {
        let rivers = result.rivers.len();
        let _ = writeln!(
            stdout,
            "Checked {rivers} river{}: {errors} error{}, {warnings} warning{}.",
            plural(rivers),
            plural(errors),
            plural(warnings),
        );
    }
    if errors > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn run(river: &Path, no_check: bool) -> ExitCode {
    if !no_check {
        let result = check_river(river);
        if result.error_count() > 0 {
            let exit = report(&result, OutputFormat::Full);
            eprintln!(
                "error: `{}` does not check; refusing to run (use --no-check to override)",
                river.display()
            );
            return exit;
        }
    }

    let steps = resolve_run_steps(river);
    let total = steps.len();
    for (index, step) in steps.iter().enumerate() {
        eprintln!("[{}/{total}] {}", index + 1, step.script.display());
        // Steps run with the directory of the river that listed them as
        // cwd, so relative artifact paths resolve consistently.
        let script = step
            .script
            .file_name()
            .map_or_else(|| step.script.clone(), PathBuf::from);
        let cwd = if step.dir.as_os_str().is_empty() {
            PathBuf::from(".")
        } else {
            step.dir.clone()
        };
        match run_python(&script, &cwd) {
            Ok(status) if status.success() => {}
            Ok(status) => {
                eprintln!(
                    "error: step `{}` failed with {status}",
                    step.script.display()
                );
                let code = status.code().unwrap_or(1);
                return ExitCode::from(u8::try_from(code).unwrap_or(1));
            }
            Err(err) => {
                eprintln!("error: cannot run `{}`: {err}", step.script.display());
                return ExitCode::from(2);
            }
        }
    }
    ExitCode::SUCCESS
}

/// Resolve the interpreter: `$RIV_PYTHON`, then `python3`, then `python`.
fn run_python(script: &Path, cwd: &Path) -> std::io::Result<std::process::ExitStatus> {
    let candidates: Vec<String> = match std::env::var("RIV_PYTHON") {
        Ok(python) => vec![python],
        Err(_) => vec!["python3".to_string(), "python".to_string()],
    };
    let mut last_err = None;
    for python in &candidates {
        match std::process::Command::new(python)
            .arg(script)
            .current_dir(cwd)
            .status()
        {
            Ok(status) => return Ok(status),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => last_err = Some(err),
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::other("no python interpreter found")))
}
