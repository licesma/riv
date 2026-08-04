//! The riv CLI: a thin dispatch layer over `riv_checker`.
//!
//! Verbs:
//! ```text
//! riv check              # validate all pipelines under the current directory
//! riv <pipeline> check   # validate one pipeline
//! riv <pipeline>         # execute one pipeline (implies check; --no-check)
//! riv <pipeline> run     # same, explicit form
//! ```

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use riv_checker::render::{OutputFormat, render_to_string};
use riv_checker::{CheckResult, check_all, check_pipeline, resolve_run_steps};

#[derive(Parser)]
#[command(
    name = "riv",
    version,
    about = "Typed data pipelines for Python",
    after_help = "`riv <pipeline.yaml>` runs one pipeline; `riv <pipeline.yaml> check` validates it."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate all pipelines under the current directory.
    Check {
        /// Output format: full, concise, json, or github.
        #[arg(long, default_value = "full")]
        output_format: String,
    },
    /// `riv <pipeline.yaml> [check|run]` (bare form runs).
    #[command(external_subcommand)]
    Pipeline(Vec<String>),
}

/// Arguments after `riv <pipeline.yaml>`, parsed with clap for real
/// `--help`/error behavior.
#[derive(Parser)]
#[command(name = "riv <pipeline.yaml>", disable_version_flag = true)]
struct PipelineCli {
    /// Skip the pre-run check (escape hatch).
    #[arg(long)]
    no_check: bool,
    /// Defaults to `run` when omitted.
    #[command(subcommand)]
    verb: Option<Verb>,
}

#[derive(Subcommand)]
enum Verb {
    /// Validate this pipeline.
    Check {
        /// Output format: full, concise, json, or github.
        #[arg(long, default_value = "full")]
        output_format: String,
    },
    /// Execute this pipeline. Refuses to start unless it type-checks.
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
        Command::Pipeline(args) => pipeline_command(&args),
    }
}

fn pipeline_command(args: &[String]) -> ExitCode {
    let pipeline = PathBuf::from(&args[0]);
    if !matches!(
        pipeline.extension().and_then(|e| e.to_str()),
        Some("yaml" | "yml")
    ) {
        let mut cmd = <Cli as clap::CommandFactory>::command();
        eprintln!(
            "{}",
            cmd.error(
                clap::error::ErrorKind::InvalidSubcommand,
                format!("unrecognized subcommand or pipeline `{}`", args[0]),
            )
            .render()
        );
        return ExitCode::from(2);
    }
    let parsed = match PipelineCli::try_parse_from(
        std::iter::once("riv <pipeline.yaml>".to_string()).chain(args[1..].iter().cloned()),
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
            report(&check_pipeline(&pipeline), format)
        }
        Some(Verb::Run { no_check }) => run(&pipeline, no_check || parsed.no_check),
        None => run(&pipeline, parsed.no_check),
    }
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
        let pipelines = result.pipelines.len();
        let _ = writeln!(
            stdout,
            "Checked {pipelines} pipeline{}: {errors} error{}, {warnings} warning{}.",
            plural(pipelines),
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

fn run(pipeline: &Path, no_check: bool) -> ExitCode {
    if !no_check {
        let result = check_pipeline(pipeline);
        if result.error_count() > 0 {
            let exit = report(&result, OutputFormat::Full);
            eprintln!(
                "error: `{}` does not check; refusing to run (use --no-check to override)",
                pipeline.display()
            );
            return exit;
        }
    }

    let steps = resolve_run_steps(pipeline);
    let total = steps.len();
    for (index, step) in steps.iter().enumerate() {
        eprintln!("[{}/{total}] {}", index + 1, step.script.display());
        // Steps run with the directory of the pipeline that listed them as
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
