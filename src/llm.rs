
use crate::{AnyError, snapshot::TechnicalSnapshot};
use crate::config::Config;

#[derive(Debug)]
pub struct LlmSignal {
    pub score: u8,
    pub explanation: String,
}

pub async fn ollama_eval(
    snapshot: &TechnicalSnapshot,
    cfg: &Config,
) -> Result<LlmSignal, AnyError> {
    let payload = serde_json::json!({
        "model": cfg.ollama_model,
        "stream": false,
        "messages": [
            { "role": "system", "content": "Return JSON { score, explanation }" },
            { "role": "user", "content": serde_json::to_string(snapshot)? }
        ]
    });

    let resp: serde_json::Value = reqwest::Client::new()
        .post(&cfg.ollama_url)
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;

    let text = resp["message"]["content"]
        .as_str()
        .ok_or("No LLM content")?;

    let json_start = text.find('{').ok_or("No JSON")?;
    let json_end = text.rfind('}').ok_or("No JSON")?;
    let raw: serde_json::Value =
        serde_json::from_str(&text[json_start..=json_end])?;

    let score = raw["score"]
        .as_f64()
        .map(|v| if v <= 10.0 { (v * 10.0).round() } else { v })
        .unwrap_or(0.0)
        .clamp(0.0, 100.0) as u8;

    let explanation = raw["explanation"]
        .as_str()
        .unwrap_or("No explanation")
        .to_string();

    Ok(LlmSignal { score, explanation })
}
