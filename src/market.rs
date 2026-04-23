use crate::AnyError;

pub async fn fetch_closes(ticker: &str) -> Result<Vec<f64>, AnyError> {
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