use anyhow::Context;

use indicatif::{ProgressBar, ProgressStyle};

use tokio::time::Duration;

use std::{borrow::Cow, sync::Arc};

use crate::command_line_args::CommandLineArgs;

const SIMPLE_PROGRESS_STYLE_TEMPLATE: &str =
    "[{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

const LIGHT_BG_PROGRESS_STYLE_TEMPLATE: & str =
    "{spinner:.blue.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.blue.bold/red}] ETA {eta_precise}";

const DARK_BG_PROGRESS_STYLE_TEMPLATE: & str =
    "{spinner:.cyan.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.cyan.bold/blue}] ETA {eta_precise}";

struct ProgressStyleInfo {
    progress_style: ProgressStyle,
    enable_spinner: bool,
}

fn choose_progress_style() -> anyhow::Result<ProgressStyleInfo> {
    let setting = std::env::var("PROGRESS_STYLE").map_or(Cow::from("default"), Cow::from);

    match setting.as_ref() {
        "simple" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(SIMPLE_PROGRESS_STYLE_TEMPLATE).unwrap(),
            enable_spinner: false,
        }),
        "light_bg" | "default" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(LIGHT_BG_PROGRESS_STYLE_TEMPLATE)
                .unwrap()
                .progress_chars("#>-"),
            enable_spinner: true,
        }),
        "dark_bg" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(DARK_BG_PROGRESS_STYLE_TEMPLATE)
                .unwrap()
                .progress_chars("#>-"),
            enable_spinner: true,
        }),
        _ => anyhow::bail!("unknown PROGRESS_STYLE: {}", setting),
    }
}

pub struct Progress {
    progress_bar: Option<ProgressBar>,
}

impl Progress {
    pub fn new(command_line_args: &CommandLineArgs) -> anyhow::Result<Arc<Self>> {
        let progress_bar = if !command_line_args.progress_bar {
            None
        } else {
            let style_info = choose_progress_style()?;

            let progress_bar = ProgressBar::new(0);
            if style_info.enable_spinner {
                progress_bar.enable_steady_tick(Duration::from_millis(100));
            }

            progress_bar.set_style(style_info.progress_style);

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
