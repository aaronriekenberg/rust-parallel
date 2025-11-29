use anyhow::Context;

use tokio::sync::Mutex;

use tracing::error;

use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::command_line_args::CommandLineArgs;

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

    pub async fn resolve_command_path<'a>(
        &self,
        command_path: Cow<'a, Path>,
    ) -> anyhow::Result<Option<Cow<'a, Path>>> {
        if !self.enabled {
            return Ok(Some(command_path));
        }

        let command_path_buf = command_path.to_path_buf();

        let mut cache = self.cache.lock().await;

        if let Some(cached_value) = cache.get(&command_path_buf) {
            return Ok(match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => Some(Cow::Owned(cached_path.to_path_buf())),
            });
        }

        let command_path_clone = command_path_buf.clone();

        let which_result = tokio::task::spawn_blocking(move || which::which(command_path_clone))
            .await
            .context("spawn_blocking error")?;

        let full_path = match which_result {
            Ok(path) => path,
            Err(e) => {
                error!("error resolving path {:?}: {}", command_path, e);
                cache.insert(command_path_buf, CacheValue::NotResolvable);
                return Ok(None);
            }
        };

        cache.insert(command_path_buf, CacheValue::Resolved(full_path.clone()));

        Ok(Some(Cow::from(full_path)))
    }
}
