use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;

use predicates::prelude::*;

pub fn rust_parallel_raw_command() -> Command {
    let mut cmd = Command::cargo_bin("rust-parallel").unwrap();
    cmd.current_dir("tests/");
    cmd
}

pub fn rust_parallel() -> assert_cmd::Command {
    assert_cmd::Command::from_std(rust_parallel_raw_command())
}

#[test]
fn runs_successfully_command_line() {
    rust_parallel()
        .arg("-c")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_command_line() {
    rust_parallel()
        .arg("-c")
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
fn runs_echo_command_line_j1() {
    rust_parallel()
        .arg("-c")
        .arg("-j1")
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(predicate::str::is_match("^A\nB\nC\n$").unwrap())
        .stderr(predicate::str::is_empty());
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
        .stdout(predicate::str::is_match("^A\nB\nC\n$").unwrap())
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
        .stdout(predicate::str::is_match("^hello\nfrom\ninput\nfile\n$").unwrap())
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
