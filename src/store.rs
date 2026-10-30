//! zvec 封装:建库 / 写入 / 向量检索。
//!
//! collection 落在本地 `.colna/index.zvec`,不入 git,可随时由 `colna index` 重建。

use crate::chunker::Chunk;
use crate::embedder::DIM;
use anyhow::{anyhow, Result};
use zvec::{
    Collection, CollectionSchema, CollectionStats, DataType, Doc, FieldSchema, IndexParams,
    MetricType, SearchQuery,
};

const COLLECTION_NAME: &str = "memory_chunks";
const REQUIRED_SCHEMA_INDEX_FIELDS: [&str; 2] = ["text", "embedding"];
const REQUIRED_STATS_INDEX_NAMES: [&str; 1] = ["embedding"];

/// 一条检索命中结果
#[derive(Debug, Clone)]
pub struct Hit {
    /// chunk 主键(用于混合检索去重 / RRF 融合)
    pub id: String,
    /// 最终排序分(越大越相关;经过 recency / MMR 后处理)
    pub score: f32,
    /// 向量距离(cosine,越小越相关);仅向量召回提供
    pub vector_score: Option<f32>,
    /// FTS 原始分(越大越相关);仅 FTS 召回提供
    pub fts_score: Option<f32>,
    /// 时效性加分
    pub recency_boost: f32,
    /// MMR 去重惩罚
    pub mmr_penalty: f32,
    /// 入链加权
    pub link_boost: f32,
    pub source_path: String,
    pub title: String,
    pub heading: String,
    /// 标题面包屑(如 `工作日志 › 沉淀`)
    pub breadcrumb: String,
    /// `section` / `summary`
    pub kind: String,
    pub tags: String,
    pub date: String,
    pub text: String,
    /// 仅候选阶段保留,用于 MMR;不输出给调用方
    pub embedding: Option<Vec<f32>>,
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
        .add_field(FieldSchema::new("breadcrumb", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("tags", DataType::String, true, 0)?)
        .add_field(FieldSchema::new("date", DataType::String, true, 0)?)
        // text 字段挂 FTS 全文索引,用于关键词检索(P1 混合检索的稀疏一侧)
        .add_indexed_field(
            "text",
            DataType::String,
            IndexParams::fts(None, None, None)?,
        )
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
    Collection::open(path, None)
        .map_err(|e| anyhow!("打开 collection 失败(先运行 `colna index`?): {e}"))
}

pub struct CollectionStatus {
    pub doc_count: u64,
    pub indexes_complete: bool,
}

/// 读取已落盘 collection 的 schema 与统计信息,用于判断索引是否完整。
pub fn collection_status(path: &str) -> Result<CollectionStatus> {
    let collection = open(path)?;
    let schema = collection
        .schema()
        .map_err(|e| anyhow!("读取 collection schema 失败: {e}"))?;
    let stats = collection
        .stats()
        .map_err(|e| anyhow!("读取 collection 统计失败: {e}"))?;
    let status = CollectionStatus {
        doc_count: stats.doc_count,
        indexes_complete: schema_has_required_indexes(&schema) && indexes_are_complete(&stats),
    };
    collection.close()?;
    Ok(status)
}

fn schema_has_required_indexes(schema: &CollectionSchema) -> bool {
    REQUIRED_SCHEMA_INDEX_FIELDS
        .iter()
        .all(|field| schema.has_index(field))
}

pub fn indexes_are_complete(stats: &CollectionStats) -> bool {
    REQUIRED_STATS_INDEX_NAMES.iter().all(|required| {
        stats.indexes.iter().any(|index| {
            index.name == *required && index.completeness.is_finite() && index.completeness >= 1.0
        })
    }) && stats
        .indexes
        .iter()
        .all(|index| index.completeness.is_finite() && index.completeness >= 1.0)
}

fn validate_collection_schema(schema: &CollectionSchema) -> Result<()> {
    let missing = REQUIRED_SCHEMA_INDEX_FIELDS
        .iter()
        .filter(|field| !schema.has_index(field))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(anyhow!("zvec schema 缺少必需索引: {}", missing.join(", ")));
    }
    Ok(())
}

pub fn validate_collection_stats(stats: &CollectionStats, expected_doc_count: u64) -> Result<()> {
    if stats.doc_count != expected_doc_count {
        return Err(anyhow!(
            "zvec 文档数量不一致: expected={expected_doc_count}, actual={}",
            stats.doc_count
        ));
    }
    if !indexes_are_complete(stats) {
        let missing = REQUIRED_STATS_INDEX_NAMES
            .iter()
            .filter(|&&required| !stats.indexes.iter().any(|index| index.name == required))
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        let details = stats
            .indexes
            .iter()
            .map(|index| format!("{}={:.6}", index.name, index.completeness))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(anyhow!(
            "zvec 索引不完整或缺失: missing=[{missing}], stats=[{details}]"
        ));
    }
    Ok(())
}

/// 持久化写入并构建向量索引,随后用官方 stats 验证完整性。
pub fn finalize_collection(
    collection: &Collection,
    expected_doc_count: u64,
) -> Result<CollectionStats> {
    collection
        .flush()
        .map_err(|e| anyhow!("刷新索引失败: {e}"))?;
    collection
        .optimize()
        .map_err(|e| anyhow!("构建向量索引失败: {e}"))?;
    collection
        .flush()
        .map_err(|e| anyhow!("持久化优化后索引失败: {e}"))?;
    let stats = collection
        .stats()
        .map_err(|e| anyhow!("读取索引统计失败: {e}"))?;
    let schema = collection
        .schema()
        .map_err(|e| anyhow!("读取索引 schema 失败: {e}"))?;
    validate_collection_schema(&schema)?;
    validate_collection_stats(&stats, expected_doc_count)?;
    Ok(stats)
}

/// 按 chunk 主键删除。NotFound 视为已删除,保证中断后的重试幂等。
pub fn delete_chunks(collection: &Collection, chunk_ids: &[String]) -> Result<usize> {
    if chunk_ids.is_empty() {
        return Ok(0);
    }
    let refs: Vec<&str> = chunk_ids.iter().map(String::as_str).collect();
    let result = collection.delete(&refs)?;
    if result.results.len() != chunk_ids.len() {
        return Err(anyhow!(
            "zvec delete 返回数量不一致: expected={}, actual={}",
            chunk_ids.len(),
            result.results.len()
        ));
    }
    let failures = result
        .results
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.success && item.code != zvec::ErrorCode::NotFound)
        .map(|(index, item)| format!("{}: {} ({})", chunk_ids[index], item.code, item.message))
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        return Err(anyhow!("zvec delete 未完整执行: {}", failures.join(", ")));
    }
    Ok(result.success_count as usize)
}

/// 把 chunks + embeddings 组装成 Doc 列表。
fn build_docs(chunks: &[Chunk], embeddings: &[Vec<f32>]) -> Result<Vec<Doc>> {
    if chunks.len() != embeddings.len() {
        return Err(anyhow!(
            "chunk 与 embedding 数量不一致: {} != {}",
            chunks.len(),
            embeddings.len()
        ));
    }

    let mut docs: Vec<Doc> = Vec::with_capacity(chunks.len());
    for (chunk, emb) in chunks.iter().zip(embeddings.iter()) {
        let mut doc = Doc::new()?;
        doc.set_pk(&chunk.id);
        doc.add_string("id", &chunk.id)?;
        doc.add_string("source_path", &chunk.source_path)?;
        doc.add_string("title", &chunk.title)?;
        doc.add_string("heading", &chunk.heading)?;
        doc.add_string("breadcrumb", &chunk.breadcrumb)?;
        doc.add_string("tags", &chunk.tags)?;
        doc.add_string("date", &chunk.date)?;
        doc.add_string("text", &chunk.text)?;
        doc.add_vector_f32("embedding", emb)?;
        docs.push(doc);
    }
    Ok(docs)
}

/// zvec 单次写入上限为 1024 条;分批写入时留余量,避免全量重建时一次 insert 超限失败。
const MAX_WRITE_BATCH: usize = 512;

fn validate_write_counts(
    operation: &str,
    expected: usize,
    success_count: u64,
    error_count: u64,
) -> Result<usize> {
    if error_count != 0 || success_count as usize != expected {
        return Err(anyhow!(
            "zvec {operation} 未完整写入: expected={expected}, success={success_count}, errors={error_count}"
        ));
    }
    Ok(success_count as usize)
}

/// 批量写入(全量重建用 insert):chunks 与 embeddings 一一对应。
/// 按 MAX_WRITE_BATCH 分批,规避 zvec「超过 1024 条」硬上限(全量重建 1000+ 块必踩)。
pub fn insert_chunks(
    collection: &Collection,
    chunks: &[Chunk],
    embeddings: &[Vec<f32>],
) -> Result<usize> {
    let docs = build_docs(chunks, embeddings)?;
    let mut total = 0usize;
    for batch in docs.chunks(MAX_WRITE_BATCH) {
        let refs: Vec<&Doc> = batch.iter().collect();
        let res = collection.insert(&refs)?;
        total += validate_write_counts("insert", batch.len(), res.success_count, res.error_count)?;
    }
    Ok(total)
}

/// 批量 upsert(增量更新用):按主键 id 覆盖或新增。同样分批,防单次变更文件过多超限。
pub fn upsert_chunks(
    collection: &Collection,
    chunks: &[Chunk],
    embeddings: &[Vec<f32>],
) -> Result<usize> {
    let docs = build_docs(chunks, embeddings)?;
    let mut total = 0usize;
    for batch in docs.chunks(MAX_WRITE_BATCH) {
        let refs: Vec<&Doc> = batch.iter().collect();
        let res = collection.upsert(&refs)?;
        total += validate_write_counts("upsert", batch.len(), res.success_count, res.error_count)?;
    }
    Ok(total)
}

const OUTPUT_FIELDS: &[&str] = &[
    "id",
    "source_path",
    "title",
    "heading",
    "breadcrumb",
    "tags",
    "date",
    "text",
];

fn doc_to_hit(r: &Doc) -> Result<Hit> {
    Ok(Hit {
        id: r.get_string("id")?.unwrap_or_default(),
        score: r.get_score(),
        vector_score: None,
        fts_score: None,
        recency_boost: 0.0,
        mmr_penalty: 0.0,
        link_boost: 0.0,
        source_path: r.get_string("source_path")?.unwrap_or_default(),
        title: r.get_string("title")?.unwrap_or_default(),
        heading: r.get_string("heading")?.unwrap_or_default(),
        breadcrumb: r.get_string("breadcrumb")?.unwrap_or_default(),
        kind: {
            let heading = r.get_string("heading")?.unwrap_or_default();
            if heading == "__summary__" {
                "summary".to_string()
            } else {
                "section".to_string()
            }
        },
        tags: r.get_string("tags")?.unwrap_or_default(),
        date: r.get_string("date")?.unwrap_or_default(),
        text: r.get_string("text")?.unwrap_or_default(),
        embedding: None,
    })
}

/// 纯向量语义检索(HNSW + cosine)。`vector_score` 为 cosine 距离,越小越相关。
/// 候选阶段保留 embedding,供 MMR 去重使用。
pub fn vector_search(collection: &Collection, query_vec: &[f32], pool: usize) -> Result<Vec<Hit>> {
    let query = SearchQuery::builder()
        .field_name("embedding")
        .vector(query_vec)
        .topk(pool as i32)
        .include_vector(true)
        .output_fields(OUTPUT_FIELDS)
        .build()?;
    let results = collection.query(&query)?;
    results
        .iter()
        .map(|doc| {
            let mut hit = doc_to_hit(doc)?;
            hit.vector_score = Some(hit.score);
            hit.score = 1.0 - hit.score;
            hit.embedding = doc.get_vector_f32("embedding").ok().flatten();
            Ok(hit)
        })
        .collect()
}

/// FTS 全文关键词检索(text 字段的 FTS 索引)。
///
/// zvec 把 FTS 与向量视作两类互斥的 query clause:一次 SearchQuery 只能二选一,
/// 且 field_name 必须指向 FTS 索引字段。这里用空向量构造,只走 FTS clause。
pub fn fts_search(collection: &Collection, query_text: &str, pool: usize) -> Result<Vec<Hit>> {
    let query = SearchQuery::builder()
        .field_name("text")
        .vector(&[]) // 占位:FTS clause 优先,向量不参与
        .topk(pool as i32)
        .fts_match_string(query_text)
        .output_fields(OUTPUT_FIELDS)
        .build()?;
    let results = collection.query(&query)?;
    results
        .iter()
        .map(|doc| {
            let mut hit = doc_to_hit(doc)?;
            hit.fts_score = Some(hit.score);
            Ok(hit)
        })
        .collect()
}

/// 混合检索:向量召回 + FTS 召回,用 RRF(Reciprocal Rank Fusion)融合排序。
///
/// 返回的 `Hit.score` 是 RRF 融合分(越大越相关);`vector_score` / `fts_score`
/// 保留原始分供调用方展示。FTS 失败时自动退化为纯向量。
pub fn hybrid_search(
    collection: &Collection,
    query_vec: &[f32],
    query_text: &str,
    pool: usize,
) -> Result<Vec<Hit>> {
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
        let entry = acc.entry(h.id.clone()).or_insert((h.clone(), 0.0));
        entry.1 += s;
        entry.0.fts_score = h.fts_score;
        if entry.0.breadcrumb.is_empty() {
            entry.0.breadcrumb = h.breadcrumb;
        }
    }

    let mut merged: Vec<Hit> = acc
        .into_values()
        .map(|(mut h, score)| {
            h.score = score;
            h
        })
        .collect();
    merged.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    merged.truncate(pool);
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Once;

    static INIT_ZVEC: Once = Once::new();
    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

    struct TestCollectionDir(PathBuf);

    impl TestCollectionDir {
        fn new() -> Self {
            INIT_ZVEC.call_once(|| zvec::initialize(None).unwrap());
            let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("colna-memo-store-test-{}-{id}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestCollectionDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sample_chunk() -> Chunk {
        Chunk {
            id: "chunk-1".to_string(),
            source_path: "note.md".to_string(),
            title: "Note".to_string(),
            heading: "Heading".to_string(),
            breadcrumb: "Note › Heading".to_string(),
            tags: String::new(),
            date: "2026-07-31".to_string(),
            text: "Text".to_string(),
            embed_text: "Note · Heading\nText".to_string(),
        }
    }

    #[test]
    fn build_docs_rejects_embedding_count_mismatch() {
        let error = build_docs(&[sample_chunk()], &[])
            .err()
            .expect("数量不一致时应返回错误");

        assert!(error.to_string().contains("数量不一致"));
    }

    #[test]
    fn rejects_partial_zvec_writes() {
        assert_eq!(validate_write_counts("insert", 2, 2, 0).unwrap(), 2);
        assert!(validate_write_counts("insert", 2, 1, 1).is_err());
        assert!(validate_write_counts("upsert", 2, 1, 0).is_err());
    }

    #[test]
    fn rejects_missing_or_incomplete_vector_index_stats() {
        let complete = CollectionStats {
            doc_count: 2,
            indexes: vec![zvec::IndexStat {
                name: "embedding".to_string(),
                completeness: 1.0,
            }],
        };
        assert!(validate_collection_stats(&complete, 2).is_ok());
        assert!(validate_collection_stats(&complete, 1).is_err());

        let missing_vector = CollectionStats {
            doc_count: 2,
            indexes: vec![zvec::IndexStat {
                name: "other-vector".to_string(),
                completeness: 1.0,
            }],
        };
        assert!(validate_collection_stats(&missing_vector, 2).is_err());

        let incomplete = CollectionStats {
            doc_count: 2,
            indexes: vec![zvec::IndexStat {
                name: "embedding".to_string(),
                completeness: 0.5,
            }],
        };
        assert!(validate_collection_stats(&incomplete, 2).is_err());
    }

    #[test]
    fn rejects_schema_missing_required_indexes() {
        let complete = build_schema().unwrap();
        assert!(validate_collection_schema(&complete).is_ok());

        let mut missing_fts = build_schema().unwrap();
        missing_fts.drop_index("text").unwrap();
        let error = validate_collection_schema(&missing_fts).unwrap_err();
        assert!(error.to_string().contains("text"));

        let mut missing_vector = build_schema().unwrap();
        missing_vector.drop_index("embedding").unwrap();
        let error = validate_collection_schema(&missing_vector).unwrap_err();
        assert!(error.to_string().contains("embedding"));
    }

    #[test]
    #[ignore = "large zvec finalize regression"]
    fn large_collection_finalizes() {
        let sandbox = TestCollectionDir::new();
        let path = sandbox.0.join("index.zvec");
        let collection = create(path.to_str().unwrap()).unwrap();
        let chunks: Vec<Chunk> = (0..6000)
            .map(|index| Chunk {
                id: format!("chunk-{index}"),
                source_path: format!("note-{}.md", index % 400),
                title: "Large".to_string(),
                heading: if index % 17 == 0 {
                    "__summary__".to_string()
                } else {
                    format!("H{index}")
                },
                breadcrumb: "Large › Section".to_string(),
                tags: "test".to_string(),
                date: "2026-10-30".to_string(),
                text: format!("chunk text {index} with some content for the scalar store"),
                embed_text: format!("Large · Section\nchunk text {index}"),
            })
            .collect();
        let embeddings = vec![vec![0.0; DIM]; chunks.len()];
        insert_chunks(&collection, &chunks, &embeddings).unwrap();
        finalize_collection(&collection, chunks.len() as u64).unwrap();
        collection.close().unwrap();
    }

    #[test]
    fn delete_by_id_removes_docs_from_an_optimized_collection_and_is_idempotent() {
        let sandbox = TestCollectionDir::new();
        let path = sandbox.0.join("index.zvec");
        let collection = create(path.to_str().unwrap()).unwrap();
        let chunk = sample_chunk();
        let embedding = vec![0.0; DIM];

        insert_chunks(&collection, &[chunk], &[embedding]).unwrap();
        finalize_collection(&collection, 1).unwrap();
        delete_chunks(&collection, &["chunk-1".to_string()]).unwrap();
        delete_chunks(&collection, &["chunk-1".to_string()]).unwrap();
        finalize_collection(&collection, 0).unwrap();
        collection.close().unwrap();
    }
}
