/// Verilen string'in uzunluğuna göre fiyat döndürür.
/// 1 harf -> 375_000_000
/// 2 harf -> 200_000_000
/// 3 harf -> 120_000_000
/// 4 harf -> 55_000_000
/// 5+ harf -> 20_000_000
pub fn get_price_by_length(input: &str) -> u64 {
    match input.len() {
        1 => 375_000_000,
        2 => 200_000_000,
        3 => 120_000_000,
        4 => 55_000_000,
        _ => 20_000_000,
    }
}
