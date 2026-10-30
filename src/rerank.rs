//! 可选 cross-encoder 重排(首次使用会下载 BGE reranker 模型)。
//!
//! 默认关闭;`colna search --rerank` / MCP `rerank=true` 才加载。

use anyhow::{Context, Result};
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};

pub struct Reranker {
    model: TextRerank,
}

impl Reranker {
    pub fn new() -> Result<Self> {
        let model = TextRerank::try_new(
            RerankInitOptions::new(RerankerModel::BGERerankerBase)
                .with_show_download_progress(false),
        )
        .context("加载 reranker 失败")?;
        Ok(Self { model })
    }

    /// 返回 (原候选下标, 分数),按分数降序。
    pub fn rerank(
        &self,
        query: &str,
        documents: &[String],
        top_n: usize,
    ) -> Result<Vec<(usize, f32)>> {
        let results = self
            .model
            .rerank(
                query,
                documents.iter().map(String::as_str).collect(),
                false,
                None,
            )
            .context("reranker 推理失败")?;
        Ok(results
            .into_iter()
            .take(top_n)
            .map(|result| (result.index, result.score))
            .collect())
    }
}
