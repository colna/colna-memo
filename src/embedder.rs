//! 本地 embedding:fastembed + multilingual-e5-base(768 维,中英多语言)。
//!
//! E5 系列要求给文本加前缀:索引内容用 "passage: ",查询用 "query: "。

use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// embedding 向量维度(multilingual-e5-base = 768)
pub const DIM: usize = 768;

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// 首次运行会自动下载模型到本地缓存。
    pub fn new() -> Result<Self> {
        Self::with_progress(true)
    }

    /// 静默构造(MCP server 等场景:stdout 必须保持纯净 JSON,不能有进度输出)。
    pub fn new_quiet() -> Result<Self> {
        Self::with_progress(false)
    }

    fn with_progress(show: bool) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Base)
                .with_show_download_progress(show),
        )?;
        Ok(Self { model })
    }

    /// 为待索引的内容块生成向量(加 "passage: " 前缀)。
    /// batch=32:内存紧张 / 共享机器上,小批次可显著降低单批峰值内存、避免 swap 抖动拖慢全量重建
    /// (默认 256 批在 e5-base/768 维 + 已满 swap 的机器上会龟速)。
    pub fn embed_passages(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let docs: Vec<String> = texts.iter().map(|t| format!("passage: {}", t)).collect();
        let embeddings = self.model.embed(docs, Some(32))?;
        Ok(embeddings)
    }

    /// 为查询生成向量(加 "query: " 前缀)。
    pub fn embed_query(&mut self, query: &str) -> Result<Vec<f32>> {
        let q = format!("query: {}", query);
        let mut embeddings = self.model.embed(vec![q], None)?;
        Ok(embeddings.remove(0))
    }
}
