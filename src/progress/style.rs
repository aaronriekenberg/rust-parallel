use anyhow::Context;

use indicatif::ProgressStyle;

use std::{borrow::Cow, env};

const DEFAULT_PROGRESS_STYLE: &str = "default";

const SIMPLE_PROGRESS_STYLE: &str = "simple";

const SIMPLE_PROGRESS_STYLE_TEMPLATE: &str =
    "[{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

const LIGHT_BG_PROGRESS_STYLE: &str = "light_bg";

const LIGHT_BG_PROGRESS_STYLE_TEMPLATE: &str =
    "{spinner:.blue.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.blue.bold/red}] ETA {eta_precise}";

const DARK_BG_PROGRESS_STYLE: &str = "dark_bg";

const DARK_BG_PROGRESS_STYLE_TEMPLATE: &str =
    "{spinner:.cyan.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.cyan.bold/blue}] ETA {eta_precise}";

const PROGRESS_STYLE: &str = "PROGRESS_STYLE";

pub struct ProgressStyleInfo {
    pub style_name: &'static str,
    pub progress_style: ProgressStyle,
    pub enable_steady_tick: bool,
}

pub fn choose_progress_style() -> anyhow::Result<ProgressStyleInfo> {
    let setting = env::var(PROGRESS_STYLE).map_or(Cow::from(DEFAULT_PROGRESS_STYLE), Cow::from);

    match &*setting {
        SIMPLE_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            style_name: SIMPLE_PROGRESS_STYLE,
            progress_style: ProgressStyle::with_template(SIMPLE_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?,
            enable_steady_tick: false,
        }),
        LIGHT_BG_PROGRESS_STYLE | DEFAULT_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            style_name: LIGHT_BG_PROGRESS_STYLE,
            progress_style: ProgressStyle::with_template(LIGHT_BG_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?
                .progress_chars("#>-"),
            enable_steady_tick: true,
        }),
        DARK_BG_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            style_name: DARK_BG_PROGRESS_STYLE,
            progress_style: ProgressStyle::with_template(DARK_BG_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?
                .progress_chars("#>-"),
            enable_steady_tick: true,
        }),
        _ => anyhow::bail!("unknown PROGRESS_STYLE: {}", setting),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Ideas from: https://github.com/tokio-rs/tracing/pull/2647/files
    #[test]
    fn test_choose_progress_style() {
        // Restores the previous value of the `PROGRESS_STYLE` env variable when
        // dropped.
        //
        // This is done in a `Drop` implementation, rather than just resetting
        // the value at the end of the test, so that the previous value is
        // restored even if the test panics.
        struct RestoreEnvVar(Result<String, env::VarError>);
        impl Drop for RestoreEnvVar {
            fn drop(&mut self) {
                match self.0 {
                    Ok(ref var) => env::set_var(PROGRESS_STYLE, var),
                    Err(_) => env::remove_var(PROGRESS_STYLE),
                }
            }
        }

        let _saved_progress_style = RestoreEnvVar(env::var(PROGRESS_STYLE));

        env::remove_var(PROGRESS_STYLE);
        let result = choose_progress_style();
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result.style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);

        env::set_var(PROGRESS_STYLE, DEFAULT_PROGRESS_STYLE);
        let result = choose_progress_style();
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result.style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);

        env::set_var(PROGRESS_STYLE, LIGHT_BG_PROGRESS_STYLE);
        let result = choose_progress_style();
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result.style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);

        env::set_var(PROGRESS_STYLE, DARK_BG_PROGRESS_STYLE);
        let result = choose_progress_style();
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result.style_name, DARK_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);

        env::set_var(PROGRESS_STYLE, SIMPLE_PROGRESS_STYLE);
        let result = choose_progress_style();
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result.style_name, SIMPLE_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, false);

        env::set_var(PROGRESS_STYLE, "unknown");
        let result = choose_progress_style();
        assert_eq!(result.is_err(), true);
    }
}
