pub fn ema(data: &[f64], len: usize) -> Vec<f64> {
    let k = 2.0 / (len as f64 + 1.0);
    let mut out = vec![data[0]; data.len()];

    for i in 1..data.len() {
        out[i] = data[i] * k + out[i - 1] * (1.0 - k);
    }
    out
}

pub fn macd(data: &[f64]) -> (f64, f64, f64) {
    let fast = ema(data, 12);
    let slow = ema(data, 26);
    let macd: Vec<f64> = fast.iter().zip(slow.iter()).map(|(a, b)| a - b).collect();
    let signal = ema(&macd, 9);
    let i = data.len() - 1;

    (macd[i], signal[i], macd[i] - signal[i])
}

pub fn atr_pct(data: &[f64]) -> f64 {
    let returns: Vec<f64> =
        data.windows(2).map(|w| ((w[1] - w[0]).abs()) / w[0]).collect();

    returns.iter().rev().take(14).sum::<f64>() / 14.0 * 100.0
}