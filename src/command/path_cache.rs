use anyhow::Context;

use tracing::{debug, error};

use std::{cell::RefCell, collections::HashMap, path::PathBuf};

use crate::command_line_args::CommandLineArgs;

enum CacheValue {
    NotResolvable,

    Resolved(PathBuf),
}

pub struct CommandPathCache {
    cache: Option<RefCell<HashMap<PathBuf, CacheValue>>>,
}

impl CommandPathCache {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            cache: if command_line_args.disable_path_cache {
                None
            } else {
                Some(RefCell::new(HashMap::new()))
            },
        }
    }

    pub async fn resolve_command_path(
        &self,
        command_path: PathBuf,
    ) -> anyhow::Result<Option<PathBuf>> {
        let cache = match &self.cache {
            None => return Ok(Some(command_path)),
            Some(cache) => cache,
        };

        if let Some(cached_value) = cache.borrow().get(&command_path) {
            return Ok(match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => Some(cached_path.clone()),
            });
        }

        let command_path_clone = command_path.clone();

        debug!("calling which command_path={command_path:?}");

        let which_result = tokio::task::spawn_blocking(move || which::which(command_path_clone))
            .await
            .context("spawn_blocking error")?;

        let mut cache_ref = cache.borrow_mut();

        let full_path = match which_result {
            Ok(path) => path,
            Err(e) => {
                error!("error resolving path {command_path:?}: {e}");
                cache_ref.insert(command_path.clone(), CacheValue::NotResolvable);
                return Ok(None);
            }
        };

        debug!("resolved command_path={command_path:?} to full_path={full_path:?}");

        cache_ref.insert(
            command_path.clone(),
            CacheValue::Resolved(full_path.clone()),
        );

        Ok(Some(full_path))
    }
}
