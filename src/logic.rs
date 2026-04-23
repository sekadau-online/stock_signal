use crate::{snapshot::TechnicalSnapshot, config::Config};

pub fn risk_ok(s: &TechnicalSnapshot, cfg: &Config) -> bool {
    s.price > s.ema_slow && s.atr_pct < cfg.atr_max_pct
}

pub fn classify(score: u8, cfg: &Config) -> &'static str {
    if score >= cfg.deploy_score_min {
        "DEPLOY"
    } else if score >= cfg.watch_score_min {
        "WATCH"
    } else {
        "REJECT"
    }
}
