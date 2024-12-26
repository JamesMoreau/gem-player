// pub const SECONDS_PER_MINUTE: u64 = 60; // = 60 seconds
// pub const MINUTES_PER_HOUR: u64 = 60; // = 3600 seconds
// pub const HOURS_PER_DAY: u64 = 24; // = 86400 seconds

pub fn format_duration_to_mmss(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let seconds_per_minute = 60;
    let minutes = total_seconds / seconds_per_minute;
    let seconds = total_seconds % seconds_per_minute;

    format!("{}:{:02}", minutes, seconds)
}

// fn format_duration_to_hhmmss(duration: std::time::Duration) -> String {
//     let total_seconds: f64 = duration.as_secs_f64();
//     let hours =
//         total_seconds / (constants::MINUTES_PER_HOUR as f64 * constants::SECONDS_PER_MINUTE as f64);
//     let minutes =
//         (total_seconds / constants::SECONDS_PER_MINUTE as f64) % constants::MINUTES_PER_HOUR as f64;
//     let seconds = total_seconds % constants::SECONDS_PER_MINUTE as f64;
//     format!("{:.0}:{:02.0}:{:02.0}", hours, minutes, seconds)
// }