//! CLI behavior tests: one integration binary, ty-style. Temp projects,
//! `env_clear()`, inline `insta_cmd` snapshots.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use insta_cmd::assert_cmd_snapshot;
use tempfile::TempDir;

struct CliTest {
    _temp: TempDir,
    dir: PathBuf,
}

impl CliTest {
    fn new() -> CliTest {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().canonicalize().unwrap();
        CliTest { _temp: temp, dir }
    }

    fn write(&self, path: &str, contents: &str) -> &Self {
        let path = self.dir.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_riv"));
        command.current_dir(&self.dir).env_clear();
        // Steps run real Python; it needs PATH and the in-repo riv runtime.
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        command.env("PYTHONPATH", python_runtime_dir());
        command
    }
}

fn python_runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../python")
        .canonicalize()
        .unwrap()
}

fn typed_project(test: &CliTest) {
    test.write(
        "pipeline.yaml",
        "name: demo\nsteps:\n  - preprocess.py\n  - train.py\n",
    );
    test.write(
        "schemas.py",
        "from riv import Schema\n\n\nclass UsersDf(Schema): ...\n",
    );
    test.write(
        "preprocess.py",
        "from schemas import UsersDf\n\nusers = [1, 2, 3]\nUsersDf.riv_out(users, \"users.pkl\")\nprint(\"step: preprocess\")\n",
    );
    test.write(
        "train.py",
        "from schemas import UsersDf\n\nusers = UsersDf.riv_in(\"users.pkl\")\nprint(f\"step: train ({len(users)} users)\")\n",
    );
}

#[test]
fn check_clean_project() {
    let test = CliTest::new();
    typed_project(&test);
    assert_cmd_snapshot!(test.command().arg("check"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Checked 1 pipeline: 0 errors, 0 warnings.

    ----- stderr -----
    ");
}

#[test]
fn check_warns_on_unannotated_io() {
    let test = CliTest::new();
    test.write("pipeline.yaml", "name: demo\nsteps:\n  - produce.py\n");
    test.write(
        "produce.py",
        "from riv import riv_out\n\nriv_out([1], \"users.pkl\")\n",
    );
    assert_cmd_snapshot!(test.command().arg("check"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    warning[unannotated-io]: artifact `users.pkl` is produced without a schema; the checker cannot see it
      --> produce.py:3:1
       |
     3 | riv_out([1], "users.pkl")
       | ^^^^^^^^^^^^^^^^^^^^^^^^^ bare `riv_out()`
       |
       = help: attach a schema (`UsersDf.riv_out(df, path)`) or mark the decision with `Untyped.riv_out(...)`

    Checked 1 pipeline: 0 errors, 1 warning.

    ----- stderr -----
    "#);
}

#[test]
fn strict_promotes_unannotated_io_to_error() {
    let test = CliTest::new();
    test.write(
        "pipeline.yaml",
        "name: demo\nstrict: true\nsteps:\n  - produce.py\n",
    );
    test.write(
        "produce.py",
        "from riv import riv_out\n\nriv_out([1], \"users.pkl\")\n",
    );
    assert_cmd_snapshot!(test.command().arg("check"), @r#"
    success: false
    exit_code: 1
    ----- stdout -----
    error[unannotated-io]: artifact `users.pkl` is produced without a schema; the checker cannot see it
      --> produce.py:3:1
       |
     3 | riv_out([1], "users.pkl")
       | ^^^^^^^^^^^^^^^^^^^^^^^^^ bare `riv_out()`
       |
       = help: attach a schema (`UsersDf.riv_out(df, path)`) or mark the decision with `Untyped.riv_out(...)`

    Checked 1 pipeline: 1 error, 0 warnings.

    ----- stderr -----
    "#);
}

#[test]
fn concise_output_format() {
    let test = CliTest::new();
    test.write("pipeline.yaml", "name: demo\nsteps:\n  - produce.py\n");
    test.write(
        "produce.py",
        "from riv import riv_out\n\nriv_out([1], \"users.pkl\")\n",
    );
    assert_cmd_snapshot!(
        test.command().args(["check", "--output-format", "concise"]),
        @r"
    success: true
    exit_code: 0
    ----- stdout -----
    warning[unannotated-io] produce.py:3:1: artifact `users.pkl` is produced without a schema; the checker cannot see it
    Checked 1 pipeline: 0 errors, 1 warning.

    ----- stderr -----
    ");
}

#[test]
fn single_pipeline_check() {
    let test = CliTest::new();
    typed_project(&test);
    assert_cmd_snapshot!(test.command().args(["pipeline.yaml", "check"]), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Checked 1 pipeline: 0 errors, 0 warnings.

    ----- stderr -----
    ");
}

#[test]
fn run_executes_steps_in_order() {
    let test = CliTest::new();
    typed_project(&test);
    assert_cmd_snapshot!(test.command().args(["pipeline.yaml", "run"]), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    step: preprocess
    step: train (3 users)

    ----- stderr -----
    [1/2] preprocess.py
    [2/2] train.py
    ");
}

#[test]
fn bare_pipeline_runs() {
    let test = CliTest::new();
    typed_project(&test);
    assert_cmd_snapshot!(test.command().arg("pipeline.yaml"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    step: preprocess
    step: train (3 users)

    ----- stderr -----
    [1/2] preprocess.py
    [2/2] train.py
    ");
}

#[test]
fn run_refuses_pipeline_that_does_not_check() {
    let test = CliTest::new();
    test.write("pipeline.yaml", "name: demo\nsteps:\n  - train.py\n");
    test.write(
        "schemas.py",
        "from riv import Schema\n\n\nclass UsersDf(Schema): ...\n",
    );
    test.write(
        "train.py",
        "from schemas import UsersDf\n\nusers = UsersDf.riv_in(\"users.pkl\")\nprint(\"ran anyway\")\n",
    );
    assert_cmd_snapshot!(test.command().args(["pipeline.yaml", "run"]), @r#"
    success: false
    exit_code: 1
    ----- stdout -----
    error[consume-before-produce]: `users.pkl` is consumed before it is produced
      --> train.py:3:9
       |
     3 | users = UsersDf.riv_in("users.pkl")
       |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^ `users.pkl` does not exist yet
       |
       = note: no step in this pipeline produces it, and it does not exist on disk

    Checked 1 pipeline: 1 error, 0 warnings.

    ----- stderr -----
    error: `pipeline.yaml` does not check; refusing to run (use --no-check to override)
    "#);
}

#[test]
fn run_no_check_is_the_escape_hatch() {
    let test = CliTest::new();
    test.write("pipeline.yaml", "name: demo\nsteps:\n  - step.py\n");
    test.write("step.py", "print(\"unchecked step ran\")\n");
    // The pipeline is fine, but --no-check must skip checking entirely; make
    // it one that would fail check.
    test.write(
        "step.py",
        "from riv import Untyped\nimport pathlib\n\nUntyped.riv_out([1], \"blob.pkl\")\nprint(\"unchecked step ran\")\n",
    );
    test.write(
        "pipeline.yaml",
        "name: demo\nsteps:\n  - step.py\n  - ghost.py\n",
    );
    assert_cmd_snapshot!(test.command().args(["pipeline.yaml", "run", "--no-check"]), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    unchecked step ran

    ----- stderr -----
    [1/1] step.py
    ");
}

#[test]
fn run_propagates_step_failure() {
    let test = CliTest::new();
    test.write(
        "pipeline.yaml",
        "name: demo\nsteps:\n  - fail.py\n  - never.py\n",
    );
    test.write("fail.py", "import sys\n\nprint(\"failing\")\nsys.exit(3)\n");
    test.write("never.py", "print(\"must not run\")\n");
    assert_cmd_snapshot!(test.command().args(["pipeline.yaml", "run"]), @r"
    success: false
    exit_code: 3
    ----- stdout -----
    failing

    ----- stderr -----
    [1/2] fail.py
    error: step `fail.py` failed with exit status: 3
    ");
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    let test = CliTest::new();
    assert_cmd_snapshot!(test.command().arg("frobnicate"), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: unrecognized subcommand or pipeline `frobnicate`

    Usage: riv <COMMAND>

    For more information, try '--help'.
    ");
}

/// The README demo cannot rot: every project under `examples/` must both
/// check clean and run successfully.
#[test]
fn examples_check_clean_and_run() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut checked = 0;
    for entry in fs::read_dir(&examples_dir).unwrap().flatten() {
        let example = entry.path();
        if !example.is_dir() {
            continue;
        }
        let run = |args: &[&str]| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_riv"));
            command.current_dir(&example).env_clear();
            if let Some(path) = std::env::var_os("PATH") {
                command.env("PATH", path);
            }
            command.env("PYTHONPATH", python_runtime_dir());
            command.args(args).output().unwrap()
        };
        let check = run(&["check"]);
        assert!(
            check.status.success() && !String::from_utf8_lossy(&check.stdout).contains("error["),
            "`riv check` failed in {}:\n{}",
            example.display(),
            String::from_utf8_lossy(&check.stdout)
        );
        let executed = run(&["pipeline.yaml", "run"]);
        assert!(
            executed.status.success(),
            "`riv pipeline.yaml run` failed in {}:\n{}{}",
            example.display(),
            String::from_utf8_lossy(&executed.stdout),
            String::from_utf8_lossy(&executed.stderr)
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no examples found in {}",
        examples_dir.display()
    );
}
