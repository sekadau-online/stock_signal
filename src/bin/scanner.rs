use std::{fs, sync::{Arc, RwLock}, time::Duration};
use tokio::task::JoinSet;
use chrono::Local;

use stock_scanner::{
    config::load_config,
    snapshot::build_snapshot,
    llm::ollama_eval,
    logic::{risk_ok, classify},
    watcher::start_env_watcher,
};

#[tokio::main]
async fn main() -> Result<(), stock_scanner::AnyError> {
    let cfg = Arc::new(RwLock::new(load_config()));
    fs::create_dir_all(&cfg.read().unwrap().output_dir)?;

    let _watcher = start_env_watcher(cfg.clone())?;

    loop {
        let cfg_snapshot = cfg.read().unwrap().clone();
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");

        println!("\n🔍 Scanning {} stocks @ {}", cfg_snapshot.tickers.len(), ts);

        let mut tasks = JoinSet::new();

        for ticker in cfg_snapshot.tickers.clone() {
            let cfg = cfg_snapshot.clone();

            tasks.spawn(async move {
                let snap = build_snapshot(&ticker, &cfg).await?;
                let llm = ollama_eval(&snap, &cfg).await?;

                let action = if risk_ok(&snap, &cfg) {
                    classify(llm.score, &cfg)
                } else if llm.score >= cfg.watch_score_min {
                    "WATCH"
                } else {
                    "REJECT"
                };

                Ok::<_, stock_scanner::AnyError>((
                    ticker,
                    action,
                    llm.score,
                    llm.explanation,
                ))
            });
        }

        while let Some(res) = tasks.join_next().await {
            if let Ok(Ok((t, a, s, e))) = res {
                println!("{:<8} | {:<6} | {:>3} | {}", t, a, s, e);
            }
        }

        tokio::time::sleep(Duration::from_secs(cfg_snapshot.scan_interval_secs)).await;
    }
}