use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Local;
use tokio::task::JoinSet;

use stock_scanner::{
    config::load_config,
    snapshot::build_snapshot,
    llm::ollama_eval,
    logic::{risk_ok, classify},
    watcher::start_env_watcher,
};

#[tokio::main]
async fn main() -> Result<(), stock_scanner::AnyError> {
    // -------------------------------------------------
    // Load config & prepare output directory
    // -------------------------------------------------
    let cfg = Arc::new(RwLock::new(load_config()));
    fs::create_dir_all(&cfg.read().unwrap().output_dir)?;

    // Keep env watcher alive
    let _watcher = start_env_watcher(cfg.clone())?;

    // -------------------------------------------------
    // Main scan loop
    // -------------------------------------------------
    loop {
        // Snapshot config at start of scan
        let cfg_snapshot = cfg.read().unwrap().clone();

        let ts = Local::now()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let filename = format!(
            "{}/scan_{}.jsonl",
            cfg_snapshot.output_dir,
            Local::now().format("%Y%m%d_%H%M%S")
        );

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filename)?;

        println!(
            "\n🔍 Scanning {} stocks @ {}",
            cfg_snapshot.tickers.len(),
            ts
        );

        let mut tasks = JoinSet::new();

        // -------------------------------------------------
        // Spawn analysis tasks per ticker
        // -------------------------------------------------
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
                    snap,
                ))
            });
        }

        // -------------------------------------------------
        // Collect results
        // -------------------------------------------------
        while let Some(res) = tasks.join_next().await {
            if let Ok(Ok((ticker, action, score, explanation, snap))) = res {
                // Console output
                println!(
                    "{:<8} | {:<6} | {:>3} | {}",
                    ticker, action, score, explanation
                );

                // Structured JSON output
                let record = serde_json::json!({
                    "timestamp": ts,
                    "ticker": ticker,
                    "action": action,
                    "score": score,
                    "technical": {
                        "price": snap.price,
                        "ema_fast": snap.ema_fast,
                        "ema_slow": snap.ema_slow,
                        "macd": snap.macd,
                        "macd_signal": snap.macd_signal,
                        "macd_hist": snap.macd_hist,
                        "atr_pct": snap.atr_pct
                    },
                    "llm_explanation": explanation
                });

                writeln!(file, "{}", record)?;
            }
        }

        // -------------------------------------------------
        // Wait until next scan
        // -------------------------------------------------
        tokio::time::sleep(Duration::from_secs(
            cfg_snapshot.scan_interval_secs
        ))
        .await;
    }
}