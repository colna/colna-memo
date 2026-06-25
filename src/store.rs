//! zvec 封装:建库 / 写入 / 向量检索。
//!
//! collection 落在本地 `.colna/index.zvec`,不入 git,可随时由 `colna index` 重建。

use crate::chunker::Chunk;
use crate::embedder::DIM;
use anyhow::{anyhow, Result};
use zvec::{
    Collection, CollectionSchema, DataType, Doc, FieldSchema, IndexParams, MetricType, SearchQuery,
};

const COLLECTION_NAME: &str = "memory_chunks";

/// 一条检索命中结果
#[derive(Debug)]
pub struct Hit {
    pub score: f32,
    pub source_path: String,
    pub title: String,
    pub heading: String,
    pub tags: String,
    pub date: String,
    pub text: String,
}

/// 初始化 zvec 运行时(进程级,调用一次)
pub fn init() -> Result<()> {
    zvec::initialize(None).map_err(|e| anyhow!("zvec initialize 失败: {e}"))?;
    Ok(())
}

/// 关闭 zvec 运行时
pub fn shutdown() -> Result<()> {
    zvec::shutdown().map_err(|e| anyhow!("zvec shutdown 失败: {e}"))?;
    Ok(())
}

fn build_schema() -> Result<CollectionSchema> {
    let schema = CollectionSchema::builder(COLLECTION_NAME)
        .add_field(FieldSchema::new("id", DataType::String, false, 0)?)
        .add_field(FieldSchema::new("source_path", DataType::String, false, 0)?)
        .add_field(FieldSchema::new("title", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("heading", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("tags", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("date", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("text", DataType::String, false, 0)?)
        .add_vector_field(
            "embedding",
            DataType::VectorFp32,
            DIM as u32,
            IndexParams::hnsw(MetricType::Cosine, 16, 100)?,
        )
        .build()?;
    Ok(schema)
}

/// 创建一个全新的 collection(若已存在请先删目录)。
pub fn create(path: &str) -> Result<Collection> {
    let schema = build_schema()?;
    let collection = Collection::create_and_open(path, &schema, None)
        .map_err(|e| anyhow!("创建 collection 失败: {e}"))?;
    Ok(collection)
}

/// 打开已有 collection(用于检索)。
pub fn open(path: &str) -> Result<Collection> {
    Collection::open(path, None).map_err(|e| anyhow!("打开 collection 失败(先运行 `colna index`?): {e}"))
}

/// 批量写入:chunks 与 embeddings 一一对应。
pub fn insert_chunks(
    collection: &Collection,
    chunks: &[Chunk],
    embeddings: &[Vec<f32>],
) -> Result<usize> {
    let mut docs: Vec<Doc> = Vec::with_capacity(chunks.len());
    for (chunk, emb) in chunks.iter().zip(embeddings.iter()) {
        let mut doc = Doc::new()?;
        doc.set_pk(&chunk.id);
        doc.add_string("id", &chunk.id)?;
        doc.add_string("source_path", &chunk.source_path)?;
        doc.add_string("title", &chunk.title)?;
        doc.add_string("heading", &chunk.heading)?;
        doc.add_string("tags", &chunk.tags)?;
        doc.add_string("date", &chunk.date)?;
        doc.add_string("text", &chunk.text)?;
        doc.add_vector_f32("embedding", emb)?;
        docs.push(doc);
    }
    let refs: Vec<&Doc> = docs.iter().collect();
    let res = collection.insert(&refs)?;
    Ok(res.success_count as usize)
}

/// 向量检索,返回 topk 命中。
pub fn search(collection: &Collection, query_vec: &[f32], topk: usize) -> Result<Vec<Hit>> {
    let query = SearchQuery::builder()
        .field_name("embedding")
        .vector(query_vec)
        .topk(topk as i32)
        .output_fields(&["source_path", "title", "heading", "tags", "date", "text"])
        .build()?;
    let results = collection.query(&query)?;

    let mut hits = Vec::new();
    for r in results.iter() {
        hits.push(Hit {
            score: r.get_score(),
            source_path: r.get_string("source_path")?.unwrap_or_default(),
            title: r.get_string("title")?.unwrap_or_default(),
            heading: r.get_string("heading")?.unwrap_or_default(),
            tags: r.get_string("tags")?.unwrap_or_default(),
            date: r.get_string("date")?.unwrap_or_default(),
            text: r.get_string("text")?.unwrap_or_default(),
        });
    }
    Ok(hits)
}
