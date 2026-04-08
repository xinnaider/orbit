#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageStats {
    pub weekly_tokens: u64,
    pub today_tokens: u64,
    pub weekly_messages: u64,
    pub today_messages: u64,
}

#[tauri::command]
pub fn get_claude_usage_stats() -> ClaudeUsageStats {
    let empty = ClaudeUsageStats {
        weekly_tokens: 0,
        today_tokens: 0,
        weekly_messages: 0,
        today_messages: 0,
    };

    let stats_path = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("stats-cache.json"),
        None => return empty,
    };

    let content = match std::fs::read_to_string(&stats_path) {
        Ok(c) => c,
        Err(_) => return empty,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return empty,
    };

    // Compute today and 7-days-ago in YYYY-MM-DD format using std only
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_per_day: u64 = 86400;
    let today_days = now / secs_per_day;
    let today = days_to_date(today_days);
    let week_start = days_to_date(today_days.saturating_sub(6));

    let mut weekly_tokens: u64 = 0;
    let mut today_tokens: u64 = 0;
    let mut weekly_messages: u64 = 0;
    let mut today_messages: u64 = 0;

    if let Some(arr) = json.get("dailyModelTokens").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                if let Some(by_model) = entry.get("tokensByModel").and_then(|v| v.as_object()) {
                    let total: u64 = by_model.values().filter_map(|v| v.as_u64()).sum();
                    weekly_tokens += total;
                    if date == today.as_str() {
                        today_tokens = total;
                    }
                }
            }
        }
    }

    if let Some(arr) = json.get("dailyActivity").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                let msgs = entry
                    .get("messageCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                weekly_messages += msgs;
                if date == today.as_str() {
                    today_messages = msgs;
                }
            }
        }
    }

    ClaudeUsageStats {
        weekly_tokens,
        today_tokens,
        weekly_messages,
        today_messages,
    }
}

fn days_to_date(days: u64) -> String {
    // Compute YYYY-MM-DD from days since Unix epoch (1970-01-01)
    let mut remaining = days;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[tauri::command]
pub fn get_changelog() -> String {
    include_str!("../../../CHANGELOG.md").to_string()
}
