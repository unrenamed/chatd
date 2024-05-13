pub fn format_distance_to_now(seconds: i64) -> String {
    if seconds < 60 {
        format!("After {} seconds", seconds)
    } else if seconds < 60 * 60 {
        let minutes = seconds / 60;
        format!(
            "After {} minute{}",
            minutes,
            if minutes > 1 { "s" } else { "" }
        )
    } else if seconds < 60 * 60 * 24 {
        let hours = seconds / (60 * 60);
        format!("After {} hour{}", hours, if hours > 1 { "s" } else { "" })
    } else {
        let days = seconds / (60 * 60 * 24);
        format!("After {} day{}", days, if days > 1 { "s" } else { "" })
    }
}
