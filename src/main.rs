use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const OLLAMA_URL: &str = "http://localhost:11434/api/chat";
const OLLAMA_MODEL: &str = "zfujicute/OmniCoder-Qwen3.5-9B-Claude-4.6-Opus-Uncensored-v2-GGUF:latest";

// =================================================
// DATA MODELS
// =================================================

#[derive(Debug, Serialize)]
struct TechnicalSnapshot {
    ticker: String,
    price: f64,
    ema50: f64,
    ema200: f64,
    macd: f64,
    macd_signal: f64,
    macd_hist: f64,
    atr_pct: f64,
}

#[derive(Debug, Deserialize)]
struct LlmSignal {
    score: u8,
    explanation: String,
}

// =================================================
// YAHOO MARKET DATA
// =================================================

async fn fetch_closes(ticker: &str) -> Result<Vec<f64>, AnyError> {
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1y",
        ticker
    );

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (stock-signal)")
        .build()?;

    let text = client.get(url).send().await?.text().await?;
    let json: serde_json::Value = serde_json::from_str(&text)?;

    let closes = json["chart"]["result"][0]["indicators"]["quote"][0]["close"]
        .as_array()
        .ok_or("No close prices")?
        .iter()
        .filter_map(|v| v.as_f64())
        .collect::<Vec<f64>>();

    if closes.len() < 200 {
        Err("Not enough price data")?;
    }

    Ok(closes)
}

// =================================================
// INDICATORS
// =================================================

fn ema(data: &[f64], len: usize) -> Vec<f64> {
    let k = 2.0 / (len as f64 + 1.0);
    let mut out = vec![data[0]; data.len()];
    for i in 1..data.len() {
        out[i] = data[i] * k + out[i - 1] * (1.0 - k);
    }
    out
}

fn macd(data: &[f64]) -> (f64, f64, f64) {
    let fast = ema(data, 12);
    let slow = ema(data, 26);
    let macd: Vec<f64> = fast.iter().zip(slow.iter()).map(|(a, b)| a - b).collect();
    let signal = ema(&macd, 9);
    let i = data.len() - 1;
    (macd[i], signal[i], macd[i] - signal[i])
}

fn atr_pct(data: &[f64]) -> f64 {
    let returns: Vec<f64> = data
        .windows(2)
        .map(|w| ((w[1] - w[0]).abs()) / w[0])
        .collect();

    returns.iter().rev().take(14).sum::<f64>() / 14.0 * 100.0
}

// =================================================
// SNAPSHOT
// =================================================

async fn build_snapshot(ticker: &str) -> Result<TechnicalSnapshot, AnyError> {
    let prices = fetch_closes(ticker).await?;
    let price = *prices.last().unwrap();

    let ema50 = *ema(&prices, 50).last().unwrap();
    let ema200 = *ema(&prices, 200).last().unwrap();
    let (m, s, h) = macd(&prices);

    Ok(TechnicalSnapshot {
        ticker: ticker.to_string(),
        price,
        ema50,
        ema200,
        macd: m,
        macd_signal: s,
        macd_hist: h,
        atr_pct: atr_pct(&prices),
    })
}

// =================================================
// OLLAMA
// =================================================

#[derive(Serialize, Deserialize)]
struct OllamaMsg {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMsg>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaMsg,
}

async fn ollama_eval(snapshot: &TechnicalSnapshot) -> Result<LlmSignal, AnyError> {
    let req = OllamaRequest {
        model: OLLAMA_MODEL.into(),
        stream: false,
        messages: vec![
            OllamaMsg {
                role: "system".into(),
                content:
"Return ONLY JSON.
Rules:
- score must be numeric (0–10 or 0–100 accepted)
- explanation must be a string
Format:
{ \"score\": 8, \"explanation\": \"\" }"
                .into(),
            },
            OllamaMsg {
                role: "user".into(),
                content: serde_json::to_string(snapshot)?,
            },
        ],
    };

    let resp: OllamaResponse = reqwest::Client::new()
        .post(OLLAMA_URL)
        .json(&req)
        .send()
        .await?
        .json()
        .await?;

    let txt = resp.message.content;

    let start = txt.find('{').ok_or("No JSON")?;
    let end = txt.rfind('}').ok_or("No JSON")?;

    let raw: serde_json::Value = serde_json::from_str(&txt[start..=end])?;

    // ✅ SAFE SCORE NORMALIZATION
    let score = match raw["score"].as_f64() {
        Some(v) if v <= 10.0 => (v * 10.0).round() as u8,   // 0–10 scale
        Some(v) => v.round().clamp(0.0, 100.0) as u8,     // 0–100 scale
        None => 0,
    };

    let explanation = raw["explanation"]
        .as_str()
        .unwrap_or("No explanation")
        .to_string();

    Ok(LlmSignal { score, explanation })
}


// =================================================
// LOGIC
// =================================================

fn risk_ok(s: &TechnicalSnapshot) -> bool {
    s.price > s.ema200 && s.atr_pct < 3.0
}

fn classify(score: u8) -> String {
    match score {
        80..=100 => "DEPLOY",
        65..=79 => "WATCH",
        _ => "REJECT",
    }
    .to_string()
}

// =================================================
// MAIN
// =================================================

#[tokio::main]
async fn main() {
    let tickers = vec![
        "TLKM.JK", "BBCA.JK", "BMRI.JK", "BBRI.JK", "ASII.JK",
        "UNVR.JK", "ICBP.JK", "INDF.JK", "PGAS.JK", "ANTM.JK",
    ];

    println!("🔍 Scanning {} stocks...\n", tickers.len());

    let mut tasks = JoinSet::new();

    for t in tickers {
        tasks.spawn(async move {
            let snap = build_snapshot(t).await?;
            let llm = ollama_eval(&snap).await?;

            if !risk_ok(&snap) {
                return Ok::<_, AnyError>((t.to_string(), "REJECT".to_string(), llm.score, "Risk gate failed".to_string()));
            }

            Ok((t.to_string(), classify(llm.score), llm.score, llm.explanation))
        });
    }

    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(Ok((t, c, s, e))) => println!("{:<8} | {:<6} | {:>3} | {}", t, c, s, e),
            Ok(Err(e)) => eprintln!("⚠️ {}", e),
            Err(e) => eprintln!("⚠️ task: {}", e),
        }
    }
}
