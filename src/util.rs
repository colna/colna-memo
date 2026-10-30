//! 跨模块共用的小工具:北京时间、slug、日期换算、YAML 引号。

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MEMO_TIMEZONE: &str = "Asia/Shanghai";

fn date_with_format(format: &str) -> String {
    Command::new("date")
        .env("TZ", MEMO_TIMEZONE)
        .arg(format)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// 北京时间今天,`YYYY-MM-DD`。
pub fn today_beijing() -> String {
    date_with_format("+%Y-%m-%d")
}

/// 北京时间现在,`YYYY-MM-DD HH:MM`。
pub fn now_beijing() -> String {
    date_with_format("+%Y-%m-%d %H:%M")
}

/// 北京时间昨天,`YYYY-MM-DD`(macOS 用 -v-1d,Linux 回退 -d yesterday)。
pub fn yesterday_beijing() -> String {
    let mac = Command::new("date")
        .env("TZ", MEMO_TIMEZONE)
        .args(["-v-1d", "+%Y-%m-%d"])
        .output();
    if let Ok(output) = mac {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    Command::new("date")
        .env("TZ", MEMO_TIMEZONE)
        .args(["-d", "yesterday", "+%Y-%m-%d"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(today_beijing)
}

/// 标题转相对安全的文件名:保留中英文与数字,空白/连字符转 `-`。
pub fn slugify(title: &str) -> String {
    let mut s = String::with_capacity(title.len());
    for ch in title.chars() {
        if ch.is_alphanumeric() {
            s.push(ch);
        } else if ch.is_whitespace() || ch == '-' || ch == '_' {
            s.push('-');
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "note".to_string()
    } else {
        s
    }
}

/// 解析 `YYYY-MM-DD`(允许后面带时间);返回 `(年, 月, 日)`。
pub fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    let date = value.trim().get(..10)?;
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// Howard Hinnant 的 civil date → days since 1970-01-01。
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year } as i64;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    (secs + 8 * 3600) / 86400
}

/// 距今天数;未来日期返回负数。无法解析时返回 `None`。
pub fn age_days(value: &str) -> Option<i64> {
    let (year, month, day) = parse_date(value)?;
    Some(today_days() - days_from_civil(year, month, day))
}

/// YAML 单行字符串:总是加双引号并转义。
pub fn yaml_quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ");
    format!("\"{escaped}\"")
}

/// 把 `--type` 映射到 PARA 目录前缀。
pub fn type_prefix(kind: &str) -> Option<&'static str> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "inbox" => Some("00-Inbox/"),
        "project" | "projects" => Some("10-Projects/"),
        "area" | "areas" => Some("20-Areas/"),
        "resource" | "resources" => Some("30-Resources/"),
        "archive" | "archives" => Some("40-Archive/"),
        "daily" | "dailies" => Some("50-Daily/"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_dates_and_rejects_bad_ones() {
        assert_eq!(parse_date("2026-10-30"), Some((2026, 10, 30)));
        assert_eq!(parse_date("2026-10-30 12:00"), Some((2026, 10, 30)));
        assert!(parse_date("2026-13-01").is_none());
        assert!(parse_date("not-a-date").is_none());
    }

    #[test]
    fn computes_age_in_days() {
        let today = today_days();
        let (y, m, d) = parse_date(&today_beijing()).unwrap();
        assert_eq!(age_days(&format!("{y:04}-{m:02}-{d:02}")), Some(0));
        assert_eq!(age_days("1970-01-01"), Some(today));
    }

    #[test]
    fn slugifies_titles() {
        assert_eq!(slugify("Hello World"), "Hello-World");
        assert_eq!(slugify("a/b:c"), "abc");
        assert_eq!(slugify("---"), "note");
    }

    #[test]
    fn quotes_yaml_strings() {
        assert_eq!(yaml_quote("a:b"), "\"a:b\"");
        assert_eq!(yaml_quote("a\"b"), "\"a\\\"b\"");
    }
}
