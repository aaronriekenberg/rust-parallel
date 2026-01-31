use anyhow::Context;

use tracing::{debug, error};

use tokio::sync::RwLock;

use std::{collections::HashMap, path::PathBuf};

use crate::command_line_args::CommandLineArgs;

enum CacheValue {
    NotResolvable,

    Resolved(PathBuf),
}

pub struct CommandPathCache {
    cache: Option<RwLock<HashMap<PathBuf, CacheValue>>>,
}

impl CommandPathCache {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            cache: if command_line_args.disable_path_cache {
                None
            } else {
                Some(RwLock::new(HashMap::new()))
            },
        }
    }

    pub async fn resolve_command_path(
        &self,
        command_path: PathBuf,
    ) -> anyhow::Result<Option<PathBuf>> {
        let cache_ref = match &self.cache {
            None => return Ok(Some(command_path)),
            Some(cache) => cache,
        };

        let read_cache_ref = cache_ref.read().await;

        if let Some(cached_value) = read_cache_ref.get(&command_path) {
            return Ok(match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => Some(cached_path.clone()),
            });
        }

        drop(read_cache_ref);

        let mut write_cache_ref = cache_ref.write().await;

        let command_path_clone = command_path.clone();

        debug!("calling which command_path={command_path:?}");

        let which_result = tokio::task::spawn_blocking(move || which::which(command_path_clone))
            .await
            .context("spawn_blocking error")?;

        Ok(match which_result {
            Ok(full_path) => {
                debug!("resolved command_path={command_path:?} to full_path={full_path:?}");
                write_cache_ref.insert(
                    command_path.clone(),
                    CacheValue::Resolved(full_path.clone()),
                );
                Some(full_path)
            }
            Err(e) => {
                error!("error resolving path {command_path:?}: {e}");
                write_cache_ref.insert(command_path.clone(), CacheValue::NotResolvable);
                None
            }
        })
    }
}
