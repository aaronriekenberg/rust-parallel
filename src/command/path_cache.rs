use anyhow::Context;

use tokio::sync::Mutex;

use tracing::error;

use std::{collections::HashMap, path::PathBuf};

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

enum CacheValue {
    NotResolvable,

    Resolved(PathBuf),
}

pub struct CommandPathCache {
    enabled: bool,
    cache: Mutex<HashMap<PathBuf, CacheValue>>,
}

impl CommandPathCache {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            enabled: !command_line_args.disable_path_cache,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn resolve_command_path(
        &self,
        command_and_args: OwnedCommandAndArgs,
    ) -> anyhow::Result<Option<OwnedCommandAndArgs>> {
        if !self.enabled {
            return Ok(Some(command_and_args));
        }

        let mut command_and_args = command_and_args;

        let command_path = &command_and_args.command_path;

        let mut cache = self.cache.lock().await;

        if let Some(cached_value) = cache.get(command_path) {
            return Ok(match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => {
                    command_and_args.command_path.clone_from(cached_path);
                    Some(command_and_args)
                }
            });
        }

        let command_path_clone = command_path.clone();

        let which_result = tokio::task::spawn_blocking(move || which::which(command_path_clone))
            .await
            .context("spawn_blocking error")?;

        let full_path = match which_result {
            Ok(path) => path,
            Err(e) => {
                error!("error resolving path {:?}: {}", command_path, e);
                cache.insert(command_path.clone(), CacheValue::NotResolvable);
                return Ok(None);
            }
        };

        cache.insert(
            command_path.clone(),
            CacheValue::Resolved(full_path.clone()),
        );

        command_and_args.command_path = full_path;

        Ok(Some(command_and_args))
    }
}
