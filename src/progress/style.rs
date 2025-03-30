use anyhow::Context;

use indicatif::ProgressStyle;

use crate::command_line_args::CommandLineArgs;

const DEFAULT_PROGRESS_STYLE: &str = "default";

const SIMPLE_PROGRESS_STYLE: &str = "simple";

const SIMPLE_PROGRESS_STYLE_TEMPLATE: &str =
    "[{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} {wide_bar} ETA {eta_precise}";

const LIGHT_BG_PROGRESS_STYLE: &str = "light_bg";

const LIGHT_BG_PROGRESS_STYLE_TEMPLATE: &str = "{spinner:.blue.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.blue.bold/red}] ETA {eta_precise}";

const DARK_BG_PROGRESS_STYLE: &str = "dark_bg";

const DARK_BG_PROGRESS_STYLE_TEMPLATE: &str = "{spinner:.cyan.bold} [{elapsed_precise}] Commands Done/Total: {pos:>2}/{len:2} [{wide_bar:.cyan.bold/blue}] ETA {eta_precise}";

pub struct ProgressStyleInfo {
    _style_name: &'static str,
    pub progress_style: ProgressStyle,
    pub enable_steady_tick: bool,
}

pub fn choose_progress_style(
    command_line_args: &CommandLineArgs,
) -> anyhow::Result<ProgressStyleInfo> {
    let setting = match command_line_args.progress_bar_style {
        None => DEFAULT_PROGRESS_STYLE,
        Some(ref style) => style,
    };

    match setting {
        SIMPLE_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            _style_name: SIMPLE_PROGRESS_STYLE,
            progress_style: ProgressStyle::with_template(SIMPLE_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?,
            enable_steady_tick: false,
        }),
        LIGHT_BG_PROGRESS_STYLE | DEFAULT_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            _style_name: LIGHT_BG_PROGRESS_STYLE,
            progress_style: ProgressStyle::with_template(LIGHT_BG_PROGRESS_STYLE_TEMPLATE)
                .context("ProgressStyle::with_template error")?
                .progress_chars("#>-"),
            enable_steady_tick: true,
        }),
        DARK_BG_PROGRESS_STYLE => Ok(ProgressStyleInfo {
            _style_name: DARK_BG_PROGRESS_STYLE,
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

    #[test]
    fn test_choose_progress_style_no_style_specified() {
        let command_line_args = CommandLineArgs {
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result._style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);
    }

    #[test]
    fn test_choose_progress_style_default_style_specified() {
        let command_line_args = CommandLineArgs {
            progress_bar_style: Some(DEFAULT_PROGRESS_STYLE.to_string()),
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result._style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);
    }

    #[test]
    fn test_choose_progress_style_light_bg_style_specified() {
        let command_line_args = CommandLineArgs {
            progress_bar_style: Some(LIGHT_BG_PROGRESS_STYLE.to_string()),
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result._style_name, LIGHT_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);
    }

    #[test]
    fn test_choose_progress_style_dark_bg_style_specified() {
        let command_line_args = CommandLineArgs {
            progress_bar_style: Some(DARK_BG_PROGRESS_STYLE.to_string()),
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result._style_name, DARK_BG_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, true);
    }

    #[test]
    fn test_choose_progress_style_simple_style_specified() {
        let command_line_args = CommandLineArgs {
            progress_bar_style: Some(SIMPLE_PROGRESS_STYLE.to_string()),
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), false);
        let result = result.unwrap();
        assert_eq!(result._style_name, SIMPLE_PROGRESS_STYLE);
        assert_eq!(result.enable_steady_tick, false);
    }

    #[test]
    fn test_choose_progress_style_unknown_style_specified() {
        let command_line_args = CommandLineArgs {
            progress_bar_style: Some("unknown".to_string()),
            ..Default::default()
        };

        let result = choose_progress_style(&command_line_args);
        assert_eq!(result.is_err(), true);
    }
}
