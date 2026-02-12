pub fn from_amount_minor(amount_minor: i64) -> String {
    if amount_minor < 50_000 {
        "lt_500".to_string()
    } else if amount_minor < 200_000 {
        "500_2000".to_string()
    } else if amount_minor < 1_000_000 {
        "2000_10000".to_string()
    } else {
        "gt_10000".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::from_amount_minor;

    #[test]
    fn bucket_ranges() {
        assert_eq!(from_amount_minor(10_000), "lt_500");
        assert_eq!(from_amount_minor(50_000), "500_2000");
        assert_eq!(from_amount_minor(250_000), "2000_10000");
        assert_eq!(from_amount_minor(1_500_000), "gt_10000");
    }
}
