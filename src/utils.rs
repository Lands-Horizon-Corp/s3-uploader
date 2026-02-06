pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let base = 1024_f64;
    let exponent = (bytes as f64).log(base).floor() as i32;
    let exponent = exponent.min(UNITS.len() as i32 - 1);
    let value = bytes as f64 / base.powi(exponent);
    format!("{:.2} {}", value, UNITS[exponent as usize])
}
