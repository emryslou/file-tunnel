pub fn format_size(bytes: u64) -> String {
    let size_units = vec!["GB","MB","KB","B"];
    let size_units_len = size_units.len() - 1;
    let mut size = "";
    for (idx, size_unit) in size_units.into_iter().enumerate() {
        if bytes > (1<<(10 * (size_units_len - idx))) {
            size = format!("{}{}", bytes >> (10 * (size_units_len - idx)), size_unit);
            break ;
        }
    }

    size
}
