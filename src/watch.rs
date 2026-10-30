//! 轮询式索引 watcher + URL 变更监控。
//!
//! `colna watch` 适合配合 launchd/cron 或前台运行;`colna watch-url`
//! 借鉴 OpenMuse Goals 的「页面变化/价格阈值」,变化写回 Daily。

use crate::digest;
use crate::indexer;
use crate::util;
use crate::write;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Default, Serialize, Deserialize)]
struct WatchState {
    #[serde(default)]
    entries: BTreeMap<String, WatchEntry>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct WatchEntry {
    #[serde(default)]
    hash: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    checked_at: String,
}

pub fn watch_index(root: &Path, interval: u64, once: bool, digest: bool) -> Result<()> {
    loop {
        let changed = indexer::ensure(root, false, true, None)?;
        if changed {
            println!("[watch] {} 索引已更新", util::now_beijing());
            if digest {
                let today = util::today_beijing();
                if let Ok(report) = digest::analyze(root, &today) {
                    let _ = digest::write_digest(root, &report);
                    println!("[watch] digest → 00-Inbox/Digest-{today}.md");
                }
            }
        }
        if once {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(interval.max(5)));
    }
}

pub fn watch_url(
    root: &Path,
    url: &str,
    contains: Option<&str>,
    price: bool,
    note: Option<&str>,
    dry_run: bool,
) -> Result<bool> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        bail!("只支持 http:// 或 https:// URL");
    }
    let body = fetch(url)?;
    let hash = crate::state::content_hash(&body);
    let value = if let Some(needle) = contains {
        if body.contains(needle) {
            format!("contains:{needle}")
        } else {
            "absent".to_string()
        }
    } else if price {
        extract_price(&body).unwrap_or_else(|| "no-price".to_string())
    } else {
        hash.clone()
    };

    let state_path = root.join(".colna/watches.json");
    let mut state = load_state(&state_path)?;
    let previous = state.entries.get(url).cloned().unwrap_or_default();
    let changed = !previous.hash.is_empty() && (previous.hash != hash || previous.value != value);

    if changed {
        println!(
            "[watch] {url} 变化: {} → {}",
            if previous.value.is_empty() {
                "new"
            } else {
                &previous.value
            },
            value
        );
        if !dry_run {
            append_alert(
                root,
                note,
                &format!(
                    "- [ ] [watch] {url}: {} → {} ({})",
                    if previous.value.is_empty() {
                        "new"
                    } else {
                        &previous.value
                    },
                    value,
                    util::now_beijing()
                ),
            )?;
        }
    } else {
        println!("[watch] {url} 无变化");
    }

    if !dry_run {
        state.entries.insert(
            url.to_string(),
            WatchEntry {
                hash,
                value,
                checked_at: util::now_beijing(),
            },
        );
        save_state(&state_path, &state)?;
    }
    Ok(changed)
}

fn fetch(url: &str) -> Result<String> {
    let output = Command::new("curl")
        .args(["-fsSL", "--max-time", "60", url])
        .output()
        .context("执行 curl 失败(是否安装 curl?)")?;
    if !output.status.success() {
        bail!(
            "抓取失败 {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn extract_price(body: &str) -> Option<String> {
    for (index, _) in body.match_indices('$') {
        let rest = &body[index + 1..];
        let value: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == ',')
            .collect();
        if value.chars().any(|ch| ch.is_ascii_digit()) {
            return Some(format!("${value}"));
        }
    }
    None
}

fn load_state(path: &Path) -> Result<WatchState> {
    if !path.exists() {
        return Ok(WatchState::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("读取 watch 状态失败: {}", path.display()))?;
    serde_json::from_str(&content).with_context(|| "解析 watch 状态失败")
}

fn save_state(path: &Path, state: &WatchState) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn append_alert(root: &Path, note: Option<&str>, line: &str) -> Result<()> {
    let rel = note
        .map(str::to_string)
        .unwrap_or_else(|| format!("50-Daily/{}.md", util::today_beijing()));
    let mut content = write::read_note(root, &rel).unwrap_or_else(|_| {
        format!(
            "---\ntitle: \"{}\"\ndate: {}\ntags: daily\n---\n\n# {}\n\n",
            util::today_beijing(),
            util::today_beijing(),
            util::today_beijing()
        )
    });
    if let Some(start) = content.find("## 观察") {
        let after = start + "## 观察".len();
        let insert_at = content[after..]
            .find("\n## ")
            .map(|offset| after + offset)
            .unwrap_or(content.len());
        content.insert_str(insert_at, &format!("\n\n{line}\n"));
    } else {
        content.push_str(&format!("\n## 观察\n\n{line}\n"));
    }
    write::replace_file(root, &rel, &content)?;
    indexer::ensure(root, false, false, None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_first_dollar_price() {
        assert_eq!(
            extract_price("price $1,299.00 today").as_deref(),
            Some("$1,299.00")
        );
        assert_eq!(extract_price("no price"), None);
    }
}
