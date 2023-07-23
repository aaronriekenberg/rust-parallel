use indicatif::{ProgressBar, ProgressStyle};

use tokio::time::Duration;

use crate::command_line_args::CommandLineArgs;

const PROGRESS_STYLE: &str =
    "{spinner} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

pub struct Progress {
    progress_bar: Option<ProgressBar>,
}

impl Progress {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let progress_bar = if command_line_args.progress_bar {
            let progress_bar = ProgressBar::new(10);

            progress_bar.enable_steady_tick(Duration::from_millis(100));

            let style = ProgressStyle::with_template(PROGRESS_STYLE).unwrap();

            progress_bar.set_style(style);

            Some(progress_bar)
        } else {
            None
        };

        Self { progress_bar }
    }

    pub fn set_total_commands(&self, total_commands: u64) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.set_length(total_commands);
        }
    }

    pub fn command_finished(&self) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.inc(1);
        }
    }

    pub fn finish(&self) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.finish();
        }
    }
}
