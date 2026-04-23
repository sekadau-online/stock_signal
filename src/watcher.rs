use crate::config::load_config;
use crate::AnyError;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

pub fn start_env_watcher(
    cfg: Arc<RwLock<crate::config::Config>>,
) -> Result<RecommendedWatcher, AnyError> {
    let mut watcher: RecommendedWatcher = Watcher::new(
        move |_| {
            if Path::new(".env").exists() {
                let new_cfg = load_config();
                *cfg.write().unwrap() = new_cfg;
                println!("🔁 Config reloaded from .env");
            }
        },
        notify::Config::default(),
    )?;

    watcher.watch(Path::new(".env"), RecursiveMode::NonRecursive)?;
    Ok(watcher)
}