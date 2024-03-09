use anyhow::Context;

use indicatif::ProgressStyle;

use std::borrow::Cow;

const SIMPLE_PROGRESS_STYLE_TEMPLATE: &str =
    "[{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

const LIGHT_BG_PROGRESS_STYLE_TEMPLATE: &str =
    "{spinner:.blue.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.blue.bold/red}] ETA {eta_precise}";

const DARK_BG_PROGRESS_STYLE_TEMPLATE: &str =
    "{spinner:.cyan.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.cyan.bold/blue}] ETA {eta_precise}";

pub struct ProgressStyleInfo {
    pub progress_style: ProgressStyle,
    pub enable_spinner: bool,
}

pub fn choose_progress_style() -> anyhow::Result<ProgressStyleInfo> {
    let setting = std::env::var("PROGRESS_STYLE").map_or(Cow::from("default"), Cow::from);

    match setting.as_ref() {
        "simple" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(SIMPLE_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?,
            enable_spinner: false,
        }),
        "light_bg" | "default" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(LIGHT_BG_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?
                .progress_chars("#>-"),
            enable_spinner: true,
        }),
        "dark_bg" => Ok(ProgressStyleInfo {
            progress_style: ProgressStyle::with_template(DARK_BG_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?
                .progress_chars("#>-"),
            enable_spinner: true,
        }),
        _ => anyhow::bail!("unknown PROGRESS_STYLE: {}", setting),
    }
}
