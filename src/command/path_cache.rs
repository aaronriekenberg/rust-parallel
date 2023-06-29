use tokio::sync::Mutex;

use tracing::warn;

use std::collections::HashMap;

use crate::{command_line_args::CommandLineArgs, common::OwnedCommandAndArgs};

enum CacheValue {
    NotResolvable,

    Resolved(String),
}

pub struct CommandPathCache {
    enabled: bool,
    cache: Mutex<HashMap<String, CacheValue>>,
}

impl CommandPathCache {
    pub fn new(command_line_args: &CommandLineArgs) -> Self {
        Self {
            enabled: !command_line_args.disable_path_cache,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn resolve_command(
        &self,
        command_and_args: OwnedCommandAndArgs,
    ) -> Option<OwnedCommandAndArgs> {
        if !self.enabled {
            return Some(command_and_args);
        }

        let mut command_and_args = command_and_args;

        if command_and_args.len() == 0 {
            return None;
        }

        let command = &command_and_args[0];

        let mut cache = self.cache.lock().await;

        if let Some(cached_value) = cache.get(command) {
            return match cached_value {
                CacheValue::NotResolvable => None,
                CacheValue::Resolved(cached_path) => {
                    command_and_args[0] = cached_path.clone();
                    Some(command_and_args)
                }
            };
        }

        let full_path = match which::which(command) {
            Ok(path) => path,
            Err(e) => {
                warn!("error resolving path {:?}: {}", command, e);
                cache.insert(command.clone(), CacheValue::NotResolvable);
                return None;
            }
        };

        let full_path_string = full_path.to_string_lossy().to_string();

        cache.insert(
            command.clone(),
            CacheValue::Resolved(full_path_string.clone()),
        );

        command_and_args[0] = full_path_string;

        Some(command_and_args)
    }
}
