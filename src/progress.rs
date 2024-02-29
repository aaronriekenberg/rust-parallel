use anyhow::Context;

use indicatif::{ProgressBar, ProgressStyle};

use tokio::time::Duration;

use std::sync::Arc;

use crate::command_line_args::CommandLineArgs;

const PROGRESS_STYLE: &str =
    "{spinner} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

pub struct Progress {
    progress_bar: Option<ProgressBar>,
}

impl Progress {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Arc<Self>> {
        let progress_bar = if !command_line_args.progress_bar {
            None
        } else {
            let progress_bar = ProgressBar::new(0);
            progress_bar.enable_steady_tick(Duration::from_millis(100));

            let style = ProgressStyle::with_template(PROGRESS_STYLE)
                .context("ProgressStyle::with_template error")?;

            progress_bar.set_style(style);

            Some(progress_bar)
        };

        Ok(Arc::new(Self { progress_bar }))
    }

    pub fn increment_total_commands(&self, delta: usize) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.inc_length(delta.try_into().unwrap_or_default());
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
