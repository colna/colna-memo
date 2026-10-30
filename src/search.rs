//! 检索后处理:过滤、时效加权、MMR 去重、最终排序。
//!
//! store.rs 负责召回与 RRF 融合,本模块负责把候选池变成最终结果。

use crate::store::{self, Hit};
use crate::util;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use zvec::Collection;

#[derive(Debug, Default, Clone)]
pub struct Filters {
    /// 所有 tag 都必须存在(大小写不敏感)
    pub tags: Vec<String>,
    /// 相对 memory/ 的路径前缀;以 `.md` 结尾时按精确路径匹配
    pub path: Option<String>,
    /// 起始日期(含),`YYYY-MM-DD`
    pub since: Option<String>,
    /// 结束日期(含),`YYYY-MM-DD`
    pub until: Option<String>,
    /// scope 路径前缀(见 `.colna/scopes.json`)
    pub scope: Option<String>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.path.is_none() && self.since.is_none() && self.until.is_none()
    }

    pub fn matches(&self, hit: &Hit) -> bool {
        if let Some(scope) = &self.scope {
            let scope = normalize_path_prefix(scope);
            if !scope.is_empty() && !hit.source_path.starts_with(&scope) {
                return false;
            }
        }

        if let Some(path) = &self.path {
            let path = normalize_path_prefix(path);
            if !path.is_empty() {
                if path.ends_with(".md") {
                    if hit.source_path != path {
                        return false;
                    }
                } else if !hit.source_path.starts_with(&path) {
                    return false;
                }
            }
        }

        if !self.tags.is_empty() {
            let hit_tags: Vec<String> = hit
                .tags
                .split(',')
                .map(|tag| tag.trim().to_lowercase())
                .filter(|tag| !tag.is_empty())
                .collect();
            if !self
                .tags
                .iter()
                .all(|wanted| hit_tags.iter().any(|tag| tag == &wanted.to_lowercase()))
            {
                return false;
            }
        }

        let date = hit.date.get(..10).unwrap_or("").to_string();
        if self.since.is_some() || self.until.is_some() {
            if date.is_empty() {
                return false;
            }
            if let Some(since) = &self.since {
                if date.as_str() < since.as_str() {
                    return false;
                }
            }
            if let Some(until) = &self.until {
                if date.as_str() > until.as_str() {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub topk: usize,
    pub hybrid: bool,
    pub filters: Filters,
    /// 时效加分权重(相对候选池最高分);0 表示关闭
    pub recency_weight: f32,
    /// 是否启用 MMR 去重
    pub mmr: bool,
    /// 是否包含整篇摘要块
    pub include_summary: bool,
    /// 入链加权权重(相对候选池最高分);0 关闭
    pub link_boost: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            topk: 5,
            hybrid: true,
            filters: Filters::default(),
            recency_weight: 0.15,
            mmr: true,
            include_summary: true,
            link_boost: 0.05,
        }
    }
}

/// 执行一次检索:召回候选池 → 过滤 → 时效加权 → MMR → 截断。
pub fn query(
    collection: &Collection,
    query_vec: &[f32],
    query_text: &str,
    options: &SearchOptions,
    link_counts: Option<&BTreeMap<String, usize>>,
) -> Result<Vec<Hit>> {
    let topk = options.topk.max(1);
    let pool = if options.filters.is_empty() {
        (topk * 4).max(20)
    } else {
        // 过滤是后置的:多召回一些,避免过滤后不够 topk
        (topk * 40).clamp(400, 2000)
    };

    let mut hits = if options.hybrid {
        store::hybrid_search(collection, query_vec, query_text, pool)?
    } else {
        store::vector_search(collection, query_vec, pool)?
    };

    hits.retain(|hit| options.filters.matches(hit));
    if !options.include_summary {
        hits.retain(|hit| hit.kind != "summary");
    }
    if options.link_boost > 0.0 {
        if let Some(counts) = link_counts {
            let max_score = hits
                .iter()
                .map(|hit| hit.score)
                .fold(f32::MIN, f32::max)
                .max(1e-6);
            for hit in &mut hits {
                let count = *counts.get(&hit.source_path).unwrap_or(&0) as f32;
                let boost = options.link_boost * (count / (count + 3.0)) * max_score;
                hit.link_boost = boost;
                hit.score += boost;
            }
        }
    }
    apply_recency(&mut hits, options.recency_weight);

    if options.mmr {
        mmr_rerank(&mut hits, topk);
    } else {
        sort_by_score(&mut hits);
        hits.truncate(topk);
    }

    // 向量只用于 MMR,不返回给调用方
    for hit in &mut hits {
        hit.embedding = None;
    }
    Ok(hits)
}

/// 多 query 检索:每个 query 独立召回,按 id 累加分数后截断。
/// 用于 `COLNA_QUERY_EXPAND_CMD` 的 LLM/规则扩展。
pub fn query_expanded(
    collection: &Collection,
    embedder: &mut crate::embedder::Embedder,
    queries: &[String],
    options: &SearchOptions,
    link_counts: Option<&BTreeMap<String, usize>>,
) -> Result<Vec<Hit>> {
    if queries.len() <= 1 {
        let query_text = queries.first().map(String::as_str).unwrap_or("");
        let vector = embedder.embed_query(query_text)?;
        return query(collection, &vector, query_text, options, link_counts);
    }

    const K: f32 = 60.0;
    let mut merged: HashMap<String, (Hit, f32)> = HashMap::new();
    for query_text in queries {
        let vector = embedder.embed_query(query_text)?;
        for (rank, hit) in query(collection, &vector, query_text, options, link_counts)?
            .into_iter()
            .enumerate()
        {
            let score = 1.0 / (K + rank as f32 + 1.0);
            let entry = merged.entry(hit.id.clone()).or_insert((hit, 0.0));
            entry.1 += score;
        }
    }
    let mut hits: Vec<Hit> = merged
        .into_values()
        .map(|(mut hit, score)| {
            hit.score = score;
            hit
        })
        .collect();
    sort_by_score(&mut hits);
    hits.truncate(options.topk.max(1));
    Ok(hits)
}

/// 读取 `COLNA_QUERY_EXPAND_CMD` 生成查询变体;命令收到 query 作为 `$1`,
/// 每行一个变体。没有配置时返回原 query。
pub fn expand_queries(query: &str) -> Vec<String> {
    let mut queries = vec![query.to_string()];
    let Ok(command) = std::env::var("COLNA_QUERY_EXPAND_CMD") else {
        return queries;
    };
    if command.trim().is_empty() {
        return queries;
    }
    let output = std::process::Command::new("sh")
        .args(["-c", &format!("{command} \"$1\""), "colna-expand"])
        .arg(query)
        .output();
    let Ok(output) = output else {
        return queries;
    };
    if !output.status.success() {
        eprintln!("query 扩展命令失败,使用原 query");
        return queries;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if !line.is_empty() && !queries.iter().any(|existing| existing == line) {
            queries.push(line.to_string());
        }
        if queries.len() >= 5 {
            break;
        }
    }
    queries
}

fn normalize_path_prefix(path: &str) -> String {
    let path = path.trim().trim_start_matches("./");
    let path = path.strip_prefix("memory/").unwrap_or(path);
    path.trim_start_matches('/').to_string()
}

fn apply_recency(hits: &mut [Hit], weight: f32) {
    if weight <= 0.0 || hits.is_empty() {
        return;
    }
    let max_score = hits
        .iter()
        .map(|hit| hit.score)
        .fold(f32::MIN, f32::max)
        .max(1e-6);
    for hit in hits {
        if let Some(age_days) = util::age_days(&hit.date) {
            let recency = 1.0 / (1.0 + age_days.max(0) as f32 / 180.0);
            hit.recency_boost = weight * recency * max_score;
            hit.score += hit.recency_boost;
        }
    }
}

fn sort_by_score(hits: &mut [Hit]) {
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

fn mmr_rerank(hits: &mut Vec<Hit>, topk: usize) {
    if hits.len() <= 1 {
        sort_by_score(hits);
        hits.truncate(topk);
        return;
    }

    let min = hits.iter().map(|hit| hit.score).fold(f32::MAX, f32::min);
    let max = hits.iter().map(|hit| hit.score).fold(f32::MIN, f32::max);
    let span = max - min;
    let relevance: Vec<f32> = hits
        .iter()
        .map(|hit| {
            if span > f32::EPSILON {
                (hit.score - min) / span
            } else {
                1.0
            }
        })
        .collect();

    const LAMBDA: f32 = 0.7;
    let mut selected: Vec<usize> = Vec::new();
    let mut remaining: Vec<usize> = (0..hits.len()).collect();
    let mut scores: Vec<Option<(f32, f32)>> = vec![None; hits.len()];

    while selected.len() < topk && !remaining.is_empty() {
        let mut best_pos = 0;
        let mut best_mmr = f32::MIN;
        let mut best_penalty = 0.0;
        for (pos, &index) in remaining.iter().enumerate() {
            let max_similarity = selected
                .iter()
                .map(|&chosen| {
                    cosine_similarity(
                        hits[index].embedding.as_deref(),
                        hits[chosen].embedding.as_deref(),
                    )
                })
                .fold(0.0, f32::max);
            let penalty = (1.0 - LAMBDA) * max_similarity;
            let mmr = LAMBDA * relevance[index] - penalty;
            if mmr > best_mmr {
                best_mmr = mmr;
                best_pos = pos;
                best_penalty = penalty;
            }
        }
        let index = remaining.remove(best_pos);
        scores[index] = Some((best_mmr, best_penalty));
        selected.push(index);
    }

    let mut reranked = Vec::with_capacity(selected.len());
    for index in selected {
        let mut hit = hits[index].clone();
        if let Some((score, penalty)) = scores[index] {
            hit.score = score;
            hit.mmr_penalty = penalty;
        }
        reranked.push(hit);
    }
    *hits = reranked;
}

fn cosine_similarity(left: Option<&[f32]>, right: Option<&[f32]>) -> f32 {
    let (left, right) = match (left, right) {
        (Some(left), Some(right)) if left.len() == right.len() && !left.is_empty() => (left, right),
        _ => return 0.0,
    };
    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (a, b) in left.iter().zip(right.iter()) {
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        return 0.0;
    }
    (dot / (left_norm.sqrt() * right_norm.sqrt())).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, tags: &str, date: &str, score: f32) -> Hit {
        Hit {
            id: path.to_string(),
            score,
            vector_score: None,
            fts_score: None,
            recency_boost: 0.0,
            mmr_penalty: 0.0,
            link_boost: 0.0,
            source_path: path.to_string(),
            title: path.to_string(),
            heading: String::new(),
            breadcrumb: String::new(),
            kind: "section".to_string(),
            tags: tags.to_string(),
            date: date.to_string(),
            text: String::new(),
            embedding: None,
        }
    }

    #[test]
    fn filters_by_path_tags_and_date() {
        let filters = Filters {
            tags: vec!["rust".to_string(), "kb".to_string()],
            path: Some("10-Projects/".to_string()),
            since: Some("2026-01-01".to_string()),
            until: Some("2026-12-31".to_string()),
            scope: None,
        };

        assert!(filters.matches(&hit("10-Projects/a.md", "rust, kb", "2026-10-30", 1.0)));
        assert!(!filters.matches(&hit("20-Areas/a.md", "rust, kb", "2026-10-30", 1.0)));
        assert!(!filters.matches(&hit("10-Projects/a.md", "rust", "2026-10-30", 1.0)));
        assert!(!filters.matches(&hit("10-Projects/a.md", "rust, kb", "2025-10-30", 1.0)));
        assert!(!filters.matches(&hit("10-Projects/a.md", "rust, kb", "", 1.0)));
    }

    #[test]
    fn exact_md_path_filter_matches_only_that_file() {
        let filters = Filters {
            path: Some("10-Projects/a.md".to_string()),
            ..Filters::default()
        };
        assert!(filters.matches(&hit("10-Projects/a.md", "", "", 1.0)));
        assert!(!filters.matches(&hit("10-Projects/a.md.bak", "", "", 1.0)));
    }

    #[test]
    fn recency_boosts_newer_notes() {
        let mut hits = vec![
            hit("old.md", "", "2025-01-01", 1.0),
            hit("new.md", "", "2026-10-30", 1.0),
        ];
        apply_recency(&mut hits, 0.5);
        assert!(hits[1].score > hits[0].score);
        assert!(hits[1].recency_boost > hits[0].recency_boost);
    }

    #[test]
    fn mmr_deduplicates_near_identical_vectors() {
        let mut a = hit("a.md", "", "", 1.0);
        a.embedding = Some(vec![1.0, 0.0]);
        let mut b = hit("b.md", "", "", 0.7);
        b.embedding = Some(vec![1.0, 0.001]);
        let mut c = hit("c.md", "", "", 0.95);
        c.embedding = Some(vec![0.0, 1.0]);
        let mut hits = vec![a, b, c];

        mmr_rerank(&mut hits, 2);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].source_path, "a.md");
        // b 与 a 几乎相同,MMR 应优先选 c
        assert_eq!(hits[1].source_path, "c.md");
        assert!(hits[1].mmr_penalty <= f32::EPSILON);
    }

    #[test]
    fn cosine_similarity_handles_missing_or_mismatched_vectors() {
        assert_eq!(cosine_similarity(None, None), 0.0);
        assert_eq!(cosine_similarity(Some(&[1.0]), Some(&[1.0, 2.0])), 0.0);
        let sim = cosine_similarity(Some(&[1.0, 0.0]), Some(&[1.0, 0.0]));
        assert!((sim - 1.0).abs() < 1e-6);
    }
}
