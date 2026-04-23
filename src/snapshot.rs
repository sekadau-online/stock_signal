use serde::Serialize;
use crate::{AnyError, indicators, market};
use crate::config::Config;

#[derive(Debug, Serialize)]
pub struct TechnicalSnapshot {
    pub ticker: String,
    pub price: f64,
    pub ema_fast: f64,
    pub ema_slow: f64,
    pub macd: f64,
    pub macd_signal: f64,
    pub macd_hist: f64,
    pub atr_pct: f64,
}

pub async fn build_snapshot(
    ticker: &str,
    cfg: &Config,
) -> Result<TechnicalSnapshot, AnyError> {
    let prices = market::fetch_closes(ticker).await?;
    let price = *prices.last().unwrap();

    let ema_fast = *indicators::ema(&prices, cfg.ema_fast).last().unwrap();
    let ema_slow = *indicators::ema(&prices, cfg.ema_slow).last().unwrap();
    let (m, s, h) = indicators::macd(&prices);

    Ok(TechnicalSnapshot {
        ticker: ticker.into(),
        price,
        ema_fast,
        ema_slow,
        macd: m,
        macd_signal: s,
        macd_hist: h,
        atr_pct: indicators::atr_pct(&prices),
    })
}