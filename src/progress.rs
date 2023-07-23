use indicatif::{ProgressBar, ProgressStyle};

use tokio::time::Duration;

use crate::command_line_args::CommandLineArgs;

pub struct Progress {
    progress_bar: Option<ProgressBar>,
}

impl Progress {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        let progress_bar = if command_line_args.progress_bar {
            let progress_bar = ProgressBar::new(10);

            progress_bar.enable_steady_tick(Duration::from_millis(200));

            let style = ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] Commands Done/Total: {pos:>7}/{len:7} ({eta})"
            ).unwrap().progress_chars("#>-");

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
