//! 检索评测:query → 期望路径,计算 recall@k / MRR。
//!
//! 用于换 embedding、改切块、加 rerank / 过滤前的回归门槛。

use crate::embedder::Embedder;
use crate::indexer;
use crate::search::{self, Filters, SearchOptions};
use crate::store;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct EvalCase {
    pub query: String,
    #[serde(default)]
    pub expected: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub query: String,
    pub recall: f64,
    pub reciprocal_rank: f64,
    pub found_at: Option<usize>,
    pub hits: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EvalReport {
    pub results: Vec<QueryResult>,
    pub mean_recall: f64,
    pub mrr: f64,
}

pub fn load(path: &Path) -> Result<Vec<EvalCase>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("读取评测文件失败: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("解析评测 JSON 失败: {}", path.display()))
}

pub fn run(root: &Path, cases: &[EvalCase], topk: usize) -> Result<EvalReport> {
    let mut embedder = Embedder::new_quiet()?;
    indexer::ensure(root, false, true, Some(&mut embedder))?;
    let collection = store::open(&root.join(indexer::INDEX_PATH).to_string_lossy())?;

    let mut results = Vec::new();
    for case in cases {
        let query_vec = embedder.embed_query(&case.query)?;
        let hits = search::query(
            &collection,
            &query_vec,
            &case.query,
            &SearchOptions {
                topk,
                hybrid: true,
                filters: Filters {
                    tags: case.tags.clone(),
                    path: case.path.clone(),
                    since: case.since.clone(),
                    until: case.until.clone(),
                    scope: None,
                },
                recency_weight: 0.0,
                mmr: true,
                include_summary: true,
                link_boost: 0.0,
            },
            None,
        )?;
        let paths: Vec<String> = hits.iter().map(|hit| hit.source_path.clone()).collect();
        results.push(score_case(case, &paths));
    }
    collection.close().ok();

    let n = results.len().max(1) as f64;
    let mean_recall = results.iter().map(|item| item.recall).sum::<f64>() / n;
    let mrr = results.iter().map(|item| item.reciprocal_rank).sum::<f64>() / n;
    Ok(EvalReport {
        results,
        mean_recall,
        mrr,
    })
}

fn score_case(case: &EvalCase, hits: &[String]) -> QueryResult {
    let mut found_at = None;
    let mut matched = 0usize;
    for (index, path) in hits.iter().enumerate() {
        if case.expected.iter().any(|expected| expected == path) {
            if found_at.is_none() {
                found_at = Some(index + 1);
            }
            matched += 1;
        }
    }
    let expected = case.expected.len().max(1) as f64;
    QueryResult {
        query: case.query.clone(),
        recall: matched.min(case.expected.len()) as f64 / expected,
        reciprocal_rank: found_at.map(|rank| 1.0 / rank as f64).unwrap_or(0.0),
        found_at,
        hits: hits.to_vec(),
    }
}

pub fn render_text(report: &EvalReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "评测:{} 条  recall@k={:.3}  MRR={:.3}\n\n",
        report.results.len(),
        report.mean_recall,
        report.mrr
    ));
    for result in &report.results {
        let rank = result
            .found_at
            .map(|rank| format!("#{rank}"))
            .unwrap_or_else(|| "miss".to_string());
        out.push_str(&format!(
            "- [{}] recall={:.2} rr={:.2}  {}\n",
            rank, result.recall, result.reciprocal_rank, result.query
        ));
    }
    out
}

pub fn to_json(report: &EvalReport) -> serde_json::Value {
    serde_json::json!({
        "mean_recall": report.mean_recall,
        "mrr": report.mrr,
        "results": report.results.iter().map(|result| serde_json::json!({
            "query": result.query,
            "recall": result.recall,
            "reciprocal_rank": result.reciprocal_rank,
            "found_at": result.found_at,
            "hits": result.hits,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(expected: &[&str]) -> EvalCase {
        EvalCase {
            query: "q".to_string(),
            expected: expected.iter().map(|value| value.to_string()).collect(),
            tags: Vec::new(),
            path: None,
            since: None,
            until: None,
        }
    }

    #[test]
    fn computes_recall_and_reciprocal_rank() {
        let result = score_case(
            &case(&["a.md", "b.md"]),
            &["x.md".to_string(), "a.md".to_string(), "b.md".to_string()],
        );
        assert_eq!(result.found_at, Some(2));
        assert_eq!(result.recall, 1.0);
        assert!((result.reciprocal_rank - 0.5).abs() < 1e-9);

        let miss = score_case(&case(&["a.md"]), &["x.md".to_string()]);
        assert_eq!(miss.found_at, None);
        assert_eq!(miss.recall, 0.0);
    }
}
