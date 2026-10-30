//! 笔记图谱:解析 Markdown 链接 / wikilink,构建 backlinks。
//!
//! 图谱按需扫描 memory/ 下的 Markdown 构建,不落盘;用于 `colna related`、
//! `colna graph` 以及检索时的入链加权。

use crate::indexer;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Default, Clone)]
pub struct Graph {
    pub nodes: Vec<String>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn inbound(&self, path: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|edge| edge.to == path)
            .map(|edge| edge.from.as_str())
            .collect()
    }

    pub fn outbound(&self, path: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|edge| edge.from == path)
            .map(|edge| edge.to.as_str())
            .collect()
    }
}

pub fn build(root: &Path) -> Result<Graph> {
    let memory_root = root.join(indexer::MEMORY_DIR);
    let files = indexer::collect_markdown(&memory_root);
    let nodes: HashSet<String> = files.iter().map(|(rel, _)| rel.clone()).collect();
    let stems: HashMap<String, String> = files
        .iter()
        .filter_map(|(rel, _)| {
            Path::new(rel)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| (stem.to_lowercase(), rel.clone()))
        })
        .collect();

    let mut edges = Vec::new();
    for (rel, path) in &files {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("读取笔记失败: {}", path.display()))?;
        let mut seen = HashSet::new();
        for target in extract_targets(&content) {
            if let Some(target) = resolve_target(rel, &target, &nodes, &stems) {
                if target != *rel && seen.insert(target.clone()) {
                    edges.push(Edge {
                        from: rel.clone(),
                        to: target,
                    });
                }
            }
        }
    }

    let mut nodes: Vec<String> = nodes.into_iter().collect();
    nodes.sort();
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));
    Ok(Graph { nodes, edges })
}

pub fn in_degree(root: &Path) -> Result<BTreeMap<String, usize>> {
    let graph = build(root)?;
    let mut counts = BTreeMap::new();
    for edge in graph.edges {
        *counts.entry(edge.to).or_insert(0) += 1;
    }
    Ok(counts)
}

pub fn extract_targets(content: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = content.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b']' && bytes[index + 1] == b'(' {
            let start = index + 2;
            if let Some(end) = content[start..].find(')') {
                let raw = content[start..start + end].trim();
                let target = raw
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(['<', '>'])
                    .split('#')
                    .next()
                    .unwrap_or("");
                if is_local(target) {
                    targets.push(target.to_string());
                }
                index = start + end;
            }
        }
        index += 1;
    }

    let mut rest = content;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let raw = &after[..end];
            let target = raw.split('|').next().unwrap_or("").trim();
            if !target.is_empty() && is_local(target) {
                targets.push(target.to_string());
            }
            rest = &after[end + 2..];
        } else {
            break;
        }
    }
    targets
}

fn is_local(target: &str) -> bool {
    !target.is_empty()
        && !target.starts_with("http://")
        && !target.starts_with("https://")
        && !target.starts_with("mailto:")
        && !target.starts_with('#')
}

fn resolve_target(
    source_path: &str,
    target: &str,
    nodes: &HashSet<String>,
    stems: &HashMap<String, String>,
) -> Option<String> {
    let target = target.replace('\\', "/");
    let target = target.strip_prefix("memory/").unwrap_or(&target);
    let mut candidate = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else {
        let parent = Path::new(source_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        normalize_path(&format!("{parent}/{target}"))
    };
    if !candidate.ends_with(".md") {
        candidate.push_str(".md");
    }
    if nodes.contains(&candidate) {
        return Some(candidate);
    }

    // wikilink:[[Note Name]] 按文件名 stem 匹配
    let stem_key = target
        .trim_end_matches(".md")
        .rsplit('/')
        .next()
        .unwrap_or(target)
        .to_lowercase();
    stems.get(&stem_key).cloned()
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

pub fn to_json(graph: &Graph) -> serde_json::Value {
    serde_json::json!({
        "nodes": graph.nodes,
        "edges": graph.edges.iter().map(|edge| serde_json::json!({
            "from": edge.from,
            "to": edge.to,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_markdown_links_and_wikilinks() {
        let content = "see [A](a.md) and [B](../b.md#x), [[C]], [[D|alias]], [web](https://x.test)";
        let targets = extract_targets(content);
        assert!(targets.contains(&"a.md".to_string()));
        assert!(targets.contains(&"../b.md".to_string()));
        assert!(targets.contains(&"C".to_string()));
        assert!(targets.contains(&"D".to_string()));
        assert!(!targets.iter().any(|target| target.starts_with("https://")));
    }

    #[test]
    fn resolves_relative_and_wikilinks() {
        let nodes = HashSet::from([
            "10-Projects/a.md".to_string(),
            "30-Resources/b.md".to_string(),
        ]);
        let stems = HashMap::from([
            ("a".to_string(), "10-Projects/a.md".to_string()),
            ("b".to_string(), "30-Resources/b.md".to_string()),
        ]);
        assert_eq!(
            resolve_target("10-Projects/a.md", "../30-Resources/b.md", &nodes, &stems),
            Some("30-Resources/b.md".to_string())
        );
        assert_eq!(
            resolve_target("10-Projects/a.md", "B", &nodes, &stems),
            Some("30-Resources/b.md".to_string())
        );
    }
}
