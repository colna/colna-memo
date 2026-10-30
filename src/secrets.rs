//! 敏感信息扫描与脱敏:私钥块、常见 token、password/secret 赋值。
//!
//! `colna secrets` 默认只报告;`--fix` 用 `[REDACTED]` 替换并重建索引。

use crate::indexer;
use crate::write;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Finding {
    pub path: String,
    pub line: usize,
    pub kind: String,
    pub snippet: String,
}

pub fn scan(root: &Path) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for (rel, path) in indexer::collect_markdown_all(&root.join(indexer::MEMORY_DIR)) {
        let content = std::fs::read_to_string(&path)?;
        for (index, line) in content.lines().enumerate() {
            if let Some(kind) = detect(line) {
                findings.push(Finding {
                    path: rel.clone(),
                    line: index + 1,
                    kind,
                    snippet: truncate(line.trim(), 120),
                });
            }
        }
    }
    Ok(findings)
}

pub fn fix(root: &Path, findings: &[Finding]) -> Result<usize> {
    let mut by_path: std::collections::BTreeMap<String, Vec<&Finding>> =
        std::collections::BTreeMap::new();
    for finding in findings {
        by_path
            .entry(finding.path.clone())
            .or_default()
            .push(finding);
    }
    let mut changed = 0;
    for (rel, items) in by_path {
        let content = write::read_note(root, &rel)?;
        let mut next = redact_content(&content, &items);
        if next != content {
            write::replace_file(root, &rel, &next)?;
            changed += 1;
        }
        next.clear();
    }
    if changed > 0 {
        indexer::ensure(root, false, false, None)?;
    }
    Ok(changed)
}

fn redact_content(content: &str, findings: &[&Finding]) -> String {
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let mut skip_until_end = false;
    for line in &mut lines {
        if skip_until_end {
            if line.contains("-----END") && line.contains("PRIVATE KEY-----") {
                skip_until_end = false;
            }
            *line = String::new();
            continue;
        }
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
            *line = "[REDACTED PRIVATE KEY]".to_string();
            skip_until_end = true;
            continue;
        }
        if let Some((start, end)) = find_token(line) {
            line.replace_range(start..end, "[REDACTED]");
        }
        if let Some((start, end)) = find_assignment_value(line) {
            line.replace_range(start..end, "[REDACTED]");
        }
    }
    // 私钥块之后可能留下空行;保留原文件结构即可
    let _ = findings;
    lines.join("\n")
}

fn detect(line: &str) -> Option<String> {
    if line.contains("-----BEGIN") && line.contains("PRIVATE KEY-----") {
        return Some("private-key".to_string());
    }
    if let Some((start, end)) = find_token(line) {
        return Some(if line[start..end].starts_with("AKIA") {
            "aws-key".to_string()
        } else if line[start..end].starts_with("sk-") {
            "api-key".to_string()
        } else {
            "token".to_string()
        });
    }
    if find_assignment_value(line).is_some() {
        return Some("credential-assignment".to_string());
    }
    None
}

fn find_token(line: &str) -> Option<(usize, usize)> {
    for marker in ["AKIA", "ghp_", "github_pat_", "sk-"] {
        if let Some(start) = line.find(marker) {
            let allowed = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
            let end = start
                + line[start..]
                    .chars()
                    .take_while(|ch| allowed(*ch))
                    .map(char::len_utf8)
                    .sum::<usize>();
            let token = &line[start..end];
            let min_len = if marker == "AKIA" { 20 } else { 24 };
            if token.len() >= min_len && !token.contains("REDACTED") {
                return Some((start, end));
            }
        }
    }
    None
}

fn find_assignment_value(line: &str) -> Option<(usize, usize)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let mut rest = &line[trimmed_start..];
    let mut offset = trimmed_start;
    if let Some(stripped) = rest.strip_prefix("- ").or_else(|| rest.strip_prefix("* ")) {
        offset += rest.len() - stripped.len();
        rest = stripped;
    }
    let lower = rest.to_lowercase();
    let keys = ["password", "passwd", "secret", "token", "api_key", "apikey"];
    for key in keys {
        if !lower.starts_with(key) {
            continue;
        }
        let after_key = &rest[key.len()..];
        let after_trimmed = after_key.trim_start();
        let separator = after_trimmed.chars().next()?;
        if separator != ':' && separator != '=' {
            continue;
        }
        let value_part = &after_trimmed[1..];
        let value = value_part.trim_start();
        if value.is_empty() || value.starts_with("[REDACTED]") || value.starts_with('#') {
            continue;
        }
        let value_start = offset
            + key.len()
            + (after_key.len() - after_trimmed.len())
            + 1
            + (value_part.len() - value.len());
        let value_end = value_start
            + value
                .chars()
                .take_while(|ch| !ch.is_whitespace() && *ch != '"' && *ch != '\'')
                .map(char::len_utf8)
                .sum::<usize>();
        let raw = &line[value_start..value_end];
        if raw.len() >= 6 {
            return Some((value_start, value_end));
        }
    }
    None
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        value.chars().take(max_chars).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_secrets() {
        assert!(detect("aws = AKIAIOSFODNN7EXAMPLE").is_some());
        assert!(detect("token: ghp_123456789012345678901234567890123456").is_some());
        assert!(detect("password: hunter2hunter2").is_some());
        assert!(detect("普通文本 password 字段说明").is_none());
    }

    #[test]
    fn redacts_assignment_values() {
        let content = "password: hunter2hunter2\napi_key = sk-123456789012345678901234\n";
        let next = redact_content(content, &[]);
        assert!(!next.contains("hunter2hunter2"));
        assert!(!next.contains("sk-123456789012345678901234"));
        assert!(next.contains("[REDACTED]"));
    }
}
