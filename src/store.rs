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
#[derive(Debug, Clone)]
pub struct Hit {
    /// chunk 主键(用于混合检索去重 / RRF 融合)
    pub id: String,
    /// 打分。纯向量模式下是 cosine 距离(越小越相关);
    /// 混合模式下是 RRF 融合分(越大越相关)。
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
        // text 字段挂 FTS 全文索引,用于关键词检索(P1 混合检索的稀疏一侧)
        .add_indexed_field("text", DataType::String, IndexParams::fts(None, None, None)?)
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

/// 删除某个源文件对应的全部 chunk(增量更新时,先删旧块再写新块)。
pub fn delete_by_source(collection: &Collection, source_path: &str) -> Result<()> {
    // source_path 由相对路径构成,正常不含单引号;仍做一次转义以防万一。
    let escaped = source_path.replace('\'', "''");
    let filter = format!("source_path = '{}'", escaped);
    collection
        .delete_by_filter(&filter)
        .map_err(|e| anyhow!("按 source_path 删除失败({source_path}): {e}"))
}

/// 把 chunks + embeddings 组装成 Doc 列表。
fn build_docs(chunks: &[Chunk], embeddings: &[Vec<f32>]) -> Result<Vec<Doc>> {
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
    Ok(docs)
}

/// 批量写入(全量重建用 insert):chunks 与 embeddings 一一对应。
pub fn insert_chunks(
    collection: &Collection,
    chunks: &[Chunk],
    embeddings: &[Vec<f32>],
) -> Result<usize> {
    let docs = build_docs(chunks, embeddings)?;
    let refs: Vec<&Doc> = docs.iter().collect();
    let res = collection.insert(&refs)?;
    Ok(res.success_count as usize)
}

/// 批量 upsert(增量更新用):按主键 id 覆盖或新增。
pub fn upsert_chunks(
    collection: &Collection,
    chunks: &[Chunk],
    embeddings: &[Vec<f32>],
) -> Result<usize> {
    let docs = build_docs(chunks, embeddings)?;
    let refs: Vec<&Doc> = docs.iter().collect();
    let res = collection.upsert(&refs)?;
    Ok(res.success_count as usize)
}

const OUTPUT_FIELDS: &[&str] = &["id", "source_path", "title", "heading", "tags", "date", "text"];

fn doc_to_hit(r: &Doc) -> Result<Hit> {
    Ok(Hit {
        id: r.get_string("id")?.unwrap_or_default(),
        score: r.get_score(),
        source_path: r.get_string("source_path")?.unwrap_or_default(),
        title: r.get_string("title")?.unwrap_or_default(),
        heading: r.get_string("heading")?.unwrap_or_default(),
        tags: r.get_string("tags")?.unwrap_or_default(),
        date: r.get_string("date")?.unwrap_or_default(),
        text: r.get_string("text")?.unwrap_or_default(),
    })
}

/// 纯向量语义检索(HNSW + cosine)。score 为 cosine 距离,越小越相关。
pub fn vector_search(collection: &Collection, query_vec: &[f32], topk: usize) -> Result<Vec<Hit>> {
    let query = SearchQuery::builder()
        .field_name("embedding")
        .vector(query_vec)
        .topk(topk as i32)
        .output_fields(OUTPUT_FIELDS)
        .build()?;
    let results = collection.query(&query)?;
    results.iter().map(doc_to_hit).collect()
}

/// FTS 全文关键词检索(text 字段的 FTS 索引)。
///
/// zvec 把 FTS 与向量视作两类互斥的 query clause:一次 SearchQuery 只能二选一,
/// 且 field_name 必须指向 FTS 索引字段。这里用空向量构造,只走 FTS clause。
pub fn fts_search(collection: &Collection, query_text: &str, topk: usize) -> Result<Vec<Hit>> {
    let query = SearchQuery::builder()
        .field_name("text")
        .vector(&[]) // 占位:FTS clause 优先,向量不参与
        .topk(topk as i32)
        .fts_match_string(query_text)
        .output_fields(OUTPUT_FIELDS)
        .build()?;
    let results = collection.query(&query)?;
    results.iter().map(doc_to_hit).collect()
}

/// 混合检索:向量召回 + FTS 召回,用 RRF(Reciprocal Rank Fusion)融合排序。
///
/// 返回的 Hit.score 是 RRF 融合分(越大越相关)。FTS 失败时自动退化为纯向量。
pub fn hybrid_search(
    collection: &Collection,
    query_vec: &[f32],
    query_text: &str,
    topk: usize,
) -> Result<Vec<Hit>> {
    // 各路多召回一些候选,给融合留空间
    let pool = (topk * 4).max(20);
    let vec_hits = vector_search(collection, query_vec, pool)?;
    // FTS 对中文依赖分词器,可能无结果或报错;失败不影响整体
    let fts_hits = fts_search(collection, query_text, pool).unwrap_or_default();

    const K: f32 = 60.0;
    let mut acc: std::collections::HashMap<String, (Hit, f32)> = std::collections::HashMap::new();
    for (rank, h) in vec_hits.into_iter().enumerate() {
        let s = 1.0 / (K + rank as f32 + 1.0);
        acc.entry(h.id.clone()).or_insert((h, 0.0)).1 += s;
    }
    for (rank, h) in fts_hits.into_iter().enumerate() {
        let s = 1.0 / (K + rank as f32 + 1.0);
        acc.entry(h.id.clone()).or_insert((h, 0.0)).1 += s;
    }

    let mut merged: Vec<Hit> = acc
        .into_values()
        .map(|(mut h, score)| {
            h.score = score;
            h
        })
        .collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(topk);
    Ok(merged)
}
