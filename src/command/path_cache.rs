use anyhow::Context;

use tokio::sync::Mutex;

use tracing::{debug, error};

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
    cache: Option<Mutex<HashMap<PathBuf, CacheValue>>>,
}

impl CommandPathCache {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            cache: if command_line_args.disable_path_cache {
                None
            } else {
                Some(Mutex::new(HashMap::new()))
            },
        }
    }

    pub async fn resolve_command_path<'a>(
        &self,
        command_path: Cow<'a, Path>,
    ) -> anyhow::Result<Option<Cow<'a, Path>>> {
        let cache = match &self.cache {
            None => return Ok(Some(command_path)),
            Some(cache) => cache,
        };

        let mut cache = cache.lock().await;

        if let Some(cached_value) = cache.get(command_path.as_ref()) {
            return Ok(match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => Some(Cow::Owned(cached_path.clone())),
            });
        }

        let command_path_clone = command_path.to_path_buf();

        debug!("calling which command_path={command_path:?}");

        let which_result = tokio::task::spawn_blocking(move || which::which(command_path_clone))
            .await
            .context("spawn_blocking error")?;

        let full_path = match which_result {
            Ok(path) => path,
            Err(e) => {
                error!("error resolving path {command_path:?}: {e}");
                cache.insert(command_path.to_path_buf(), CacheValue::NotResolvable);
                return Ok(None);
            }
        };

        debug!("resolved command_path={command_path:?} to full_path={full_path:?}");

        cache.insert(
            command_path.to_path_buf(),
            CacheValue::Resolved(full_path.clone()),
        );

        Ok(Some(Cow::from(full_path)))
    }
}
