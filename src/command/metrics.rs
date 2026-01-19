use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::process::ChildProcessExecutionError;

const ORDERING: Ordering = Ordering::SeqCst;

#[derive(Debug, Default)]
pub struct CommandMetrics {
    commands_run: AtomicU64,
    error_occurred: AtomicBool,
    spawn_errors: AtomicU64,
    timeouts: AtomicU64,
    io_errors: AtomicU64,
    exit_status_errors: AtomicU64,
}

impl CommandMetrics {
    pub fn increment_commands_run(&self) {
        self.commands_run.fetch_add(1, ORDERING);
    }

    fn commands_run(&self) -> u64 {
        self.commands_run.load(ORDERING)
    }

    pub fn error_occurred(&self) -> bool {
        self.error_occurred.load(ORDERING)
    }

    fn set_error_occurred(&self) {
        self.error_occurred.store(true, ORDERING);
    }

    fn total_failures(&self) -> u64 {
        self.spawn_errors() + self.timeouts() + self.io_errors() + self.exit_status_errors()
    }

    pub fn increment_spawn_errors(&self) {
        self.set_error_occurred();
        self.spawn_errors.fetch_add(1, ORDERING);
    }

    fn spawn_errors(&self) -> u64 {
        self.spawn_errors.load(ORDERING)
    }

    pub fn handle_child_process_execution_error(&self, error: ChildProcessExecutionError) {
        match error {
            ChildProcessExecutionError::IOError(_) => self.increment_io_errors(),
            ChildProcessExecutionError::Timeout(_) => self.increment_timeouts(),
        }
    }

    fn increment_timeouts(&self) {
        self.set_error_occurred();
        self.timeouts.fetch_add(1, ORDERING);
    }

    fn timeouts(&self) -> u64 {
        self.timeouts.load(ORDERING)
    }

    fn increment_io_errors(&self) {
        self.set_error_occurred();
        self.io_errors.fetch_add(1, ORDERING);
    }

    fn io_errors(&self) -> u64 {
        self.io_errors.load(ORDERING)
    }

    pub fn increment_exit_status_errors(&self) {
        self.set_error_occurred();
        self.exit_status_errors.fetch_add(1, ORDERING);
    }

    fn exit_status_errors(&self) -> u64 {
        self.exit_status_errors.load(ORDERING)
    }
}

impl std::fmt::Display for CommandMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "commands_run={} total_failures={} spawn_errors={} timeouts={} io_errors={} exit_status_errors={}",
            self.commands_run(),
            self.total_failures(),
            self.spawn_errors(),
            self.timeouts(),
            self.io_errors(),
            self.exit_status_errors(),
        )
    }
}
