//! `colna clip <url>`:网页正文 → Markdown → `00-Inbox/`。
//!
//! 抽取顺序:`COLNA_CLIP_CMD` → 本地 `defuddle` → `r.jina.ai`。
//! 使用 `r.jina.ai` 时 URL 会发给该服务;需要完全本地时安装 defuddle
//! 或设置 `COLNA_CLIP_CMD`。

use crate::state;
use crate::util;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub struct Fetched {
    pub title: Option<String>,
    pub markdown: String,
    pub fetched_at: String,
    pub content_hash: String,
}

pub fn fetch(url: &str) -> Result<Fetched> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        bail!("只支持 http:// 或 https:// URL");
    }

    let raw = if let Ok(command) = std::env::var("COLNA_CLIP_CMD") {
        if command.trim().is_empty() {
            extract_with_defuddle(url)?
        } else {
            run_command(&command, url)?
        }
    } else if command_exists("defuddle") {
        extract_with_defuddle(url)?
    } else {
        extract_with_jina(url)?
    };

    let markdown = normalize_markdown(&raw);
    if markdown.trim().is_empty() {
        bail!("没有抽取到正文: {url}");
    }
    let title = extract_title(&raw);
    let content_hash = state::content_hash(&markdown);

    Ok(Fetched {
        title,
        markdown,
        fetched_at: util::now_beijing(),
        content_hash,
    })
}

fn extract_with_defuddle(url: &str) -> Result<String> {
    run_args("defuddle", &["parse", url, "--md"])
}

fn extract_with_jina(url: &str) -> Result<String> {
    let jina_url = format!("https://r.jina.ai/{url}");
    let output = Command::new("curl")
        .args(["-fsSL", "--max-time", "60", &jina_url])
        .output()
        .context("执行 curl 失败(是否安装 curl?)")?;
    if !output.status.success() {
        bail!(
            "r.jina.ai 抽取失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(strip_jina_metadata(&text))
}

fn run_command(command: &str, argument: &str) -> Result<String> {
    let output = Command::new("sh")
        .args(["-c", &format!("{command} \"$1\""), "colna-clip"])
        .arg(argument)
        .output()
        .with_context(|| format!("执行抽取命令失败: {command}"))?;
    if !output.status.success() {
        bail!(
            "抽取命令失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_args(command: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .with_context(|| format!("执行 {command} 失败"))?;
    if !output.status.success() {
        bail!(
            "{command} 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {command} >/dev/null 2>&1")])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn strip_jina_metadata(text: &str) -> String {
    if let Some(index) = text.find("Markdown Content:") {
        return text[index + "Markdown Content:".len()..]
            .trim_start()
            .to_string();
    }
    text.to_string()
}

fn normalize_markdown(text: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if !in_code
            && (line.starts_with("Title:")
                || line.starts_with("URL Source:")
                || line.starts_with("Published Time:")
                || line.starts_with("Markdown Content:"))
        {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

fn extract_title(text: &str) -> Option<String> {
    for line in text.lines().take(20) {
        if let Some(title) = line.strip_prefix("Title:") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    for line in text.lines() {
        if let Some(title) = line.trim_start().strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_jina_metadata_and_keeps_body() {
        let raw =
            "Title: Demo\n\nURL Source: https://example.com\n\nMarkdown Content:\n# Hello\n\nWorld\n";
        let body = strip_jina_metadata(raw);
        assert_eq!(body.trim(), "# Hello\n\nWorld");
        assert_eq!(extract_title(raw).as_deref(), Some("Demo"));
    }

    #[test]
    fn normalizes_metadata_lines_outside_code_blocks() {
        let text = "Title: Demo\n\n```\nTitle: keep\n```\n";
        let normalized = normalize_markdown(text);
        assert!(!normalized.contains("Title: Demo"));
        assert!(normalized.contains("Title: keep"));
    }

    #[test]
    fn falls_back_to_first_h1() {
        assert_eq!(extract_title("\n# Hello\n").as_deref(), Some("Hello"));
    }
}
