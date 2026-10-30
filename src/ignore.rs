//! `.colnaignore`:按路径前缀 / 后缀 / 简单通配符排除索引文件。

use std::path::Path;

pub fn load(root: &Path) -> Vec<String> {
    let path = root.join(".colnaignore");
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

pub fn is_ignored(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| matches(pattern, path))
}

fn matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
    let path = path.trim_start_matches("./");
    if pattern.is_empty() {
        return false;
    }
    if pattern.contains('*') || pattern.contains('?') {
        return wildcard(pattern, path);
    }
    path == pattern || path.starts_with(&format!("{pattern}/")) || path.ends_with(pattern)
}

fn wildcard(pattern: &str, value: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let value: Vec<char> = value.chars().collect();
    let mut p = 0;
    let mut v = 0;
    let mut star = None;
    let mut mark = 0;

    while v < value.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == value[v]) {
            p += 1;
            v += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            mark = v;
        } else if let Some(star_index) = star {
            p = star_index + 1;
            mark += 1;
            v = mark;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_prefix_suffix_and_wildcards() {
        assert!(matches("40-Archive", "40-Archive/a.md"));
        assert!(matches("*.secret.md", "notes/pass.secret.md"));
        assert!(matches(
            "10-Projects/*/draft-?.md",
            "10-Projects/a/draft-1.md"
        ));
        assert!(!matches("10-Projects", "20-Areas/a.md"));
    }
}
