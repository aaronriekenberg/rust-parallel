use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;

use predicates::prelude::*;

fn rust_parallel_raw_command() -> Command {
    let mut cmd = Command::cargo_bin("rust-parallel").unwrap();
    cmd.current_dir("tests/");
    cmd
}

fn rust_parallel() -> assert_cmd::Command {
    assert_cmd::Command::from_std(rust_parallel_raw_command())
}

#[test]
fn runs_successfully() {
    rust_parallel()
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_from_args() {
    rust_parallel()
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(predicate::str::contains("A\n").count(1))
                .and(predicate::str::contains("B\n").count(1))
                .and(predicate::str::contains("C\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_from_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(predicate::eq("A\nB\nC\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn timeout_sleep_commands_from_args() {
    rust_parallel()
        .arg("-t1")
        .arg("sleep")
        .arg(":::")
        .arg("0")
        .arg("5")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(1))
                .and(predicate::str::contains("timeout").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_stdin() {
    let stdin = r#"
        echo A
        echo B
        echo C
    "#;
    rust_parallel()
        .write_stdin(stdin)
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(predicate::str::contains("A\n").count(1))
                .and(predicate::str::contains("B\n").count(1))
                .and(predicate::str::contains("C\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_stdin_j1() {
    let stdin = r#"
        echo A
        echo B
        echo C
    "#;
    rust_parallel()
        .arg("-j1")
        .write_stdin(stdin)
        .assert()
        .success()
        .stdout(predicate::eq("A\nB\nC\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_file() {
    rust_parallel()
        .arg("-i")
        .arg("file.txt")
        .arg("echo")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(4))
                .and(predicate::str::contains("hello\n").count(1))
                .and(predicate::str::contains("from\n").count(1))
                .and(predicate::str::contains("input\n").count(1))
                .and(predicate::str::contains("file\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_file_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("file.txt")
        .arg("echo")
        .assert()
        .success()
        .stdout(predicate::eq("hello\nfrom\ninput\nfile\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn fails_j0() {
    rust_parallel()
        .arg("-j0")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "invalid value '0' for '--jobs <JOBS>'",
        ));
}

#[test]
fn fails_t0() {
    rust_parallel()
        .arg("-t0")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "invalid value '0' for '--timeout-seconds <TIMEOUT_SECONDS>'",
        ));
}

#[test]
fn runs_shell_commands_from_file_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("shell_input.txt")
        .arg("-s")
        .arg("--shell-path=./dummy_shell.sh")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function 1\ndummy_shell arg1=-c arg2=shell_function 2\ndummy_shell arg1=-c arg2=shell_function 3\ndummy_shell arg1=-c arg2=shell_function 4\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_shell_commands_from_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-s")
        .arg("--shell-path=./dummy_shell.sh")
        .arg("shell_function")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function A\ndummy_shell arg1=-c arg2=shell_function B\ndummy_shell arg1=-c arg2=shell_function C\n",
        ))
        .stderr(predicate::str::is_empty());
}
