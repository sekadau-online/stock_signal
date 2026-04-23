use std::env;
use dotenvy::dotenv;

#[derive(Clone)]
pub struct Config {
    pub tickers: Vec<String>,
    pub ollama_url: String,
    pub ollama_model: String,
    pub ema_fast: usize,
    pub ema_slow: usize,
    pub atr_max_pct: f64,
    pub deploy_score_min: u8,
    pub watch_score_min: u8,
    pub output_dir: String,
    pub scan_interval_secs: u64,
}

pub fn load_config() -> Config {
    dotenv().ok();

    Config {
        tickers: env::var("TICKERS")
            .expect("TICKERS missing")
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),

        ollama_url: env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434/api/chat".into()),

        ollama_model: env::var("OLLAMA_MODEL")
            .expect("OLLAMA_MODEL missing"),

        ema_fast: env::var("EMA_FAST").unwrap_or("50".into()).parse().unwrap(),
        ema_slow: env::var("EMA_SLOW").unwrap_or("200".into()).parse().unwrap(),

        atr_max_pct: env::var("ATR_MAX_PCT").unwrap_or("3.0".into()).parse().unwrap(),

        deploy_score_min: env::var("DEPLOY_SCORE_MIN").unwrap_or("80".into()).parse().unwrap(),
        watch_score_min: env::var("WATCH_SCORE_MIN").unwrap_or("65".into()).parse().unwrap(),

        output_dir: env::var("OUTPUT_DIR").unwrap_or("results".into()),
        scan_interval_secs: env::var("SCAN_INTERVAL_SECS")
            .unwrap_or("60".into())
            .parse()
            .unwrap(),
    }
}