use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Float32Array, Int32Array, Int64Array, RecordBatch,
    RecordBatchIterator, StringArray,
};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    connect,
    query::{ExecutableQuery, QueryBase},
    table::OptimizeAction,
    Connection, Table,
};
use osmozzz_core::{Document, OsmozzError, Result, SearchResult};
use tracing::debug;

const TABLE_NAME: &str = "documents";
const EMBEDDING_DIM: i32 = 384;

pub struct VectorStore {
    conn: Connection,
    table: Arc<tokio::sync::RwLock<Option<Table>>>,
}

impl VectorStore {
    pub async fn open(db_path: &str) -> Result<Self> {
        std::fs::create_dir_all(db_path)
            .map_err(|e| OsmozzError::Storage(format!("Create DB dir: {}", e)))?;

        let conn = connect(db_path)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("LanceDB connect: {}", e)))?;

        let store = Self {
            conn,
            table: Arc::new(tokio::sync::RwLock::new(None)),
        };
        store.ensure_table().await?;
        Ok(store)
    }

    fn schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("url", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, true),
            Field::new("content", DataType::Utf8, false),
            Field::new("checksum", DataType::Utf8, false),
            Field::new("harvested_at", DataType::Int64, false),
            Field::new("source_ts", DataType::Int64, true),
            Field::new("chunk_index", DataType::Int32, true),
            Field::new("chunk_total", DataType::Int32, true),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBEDDING_DIM,
                ),
                false,
            ),
        ]))
    }

    async fn ensure_table(&self) -> Result<()> {
        let existing = self
            .conn
            .table_names()
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("List tables: {}", e)))?;

        let schema = Self::schema();

        let table = if existing.contains(&TABLE_NAME.to_string()) {
            self.conn
                .open_table(TABLE_NAME)
                .execute()
                .await
                .map_err(|e| OsmozzError::Storage(format!("Open table: {}", e)))?
        } else {
            let empty = RecordBatch::new_empty(schema.clone());
            let reader = RecordBatchIterator::new(
                vec![Ok::<RecordBatch, ArrowError>(empty)].into_iter(),
                schema,
            );
            self.conn
                .create_table(TABLE_NAME, reader)
                .execute()
                .await
                .map_err(|e| OsmozzError::Storage(format!("Create table: {}", e)))?
        };

        *self.table.write().await = Some(table);
        Ok(())
    }

    async fn get_table(&self) -> Result<Table> {
        self.table
            .read()
            .await
            .clone()
            .ok_or_else(|| OsmozzError::NotInitialized("Vector store table".to_string()))
    }

    pub async fn upsert(&self, doc: &Document, embedding: Vec<f32>) -> Result<()> {
        let table = self.get_table().await?;
        let schema = Self::schema();

        let ids = StringArray::from(vec![doc.id.to_string()]);
        let sources = StringArray::from(vec![doc.source.to_string()]);
        let urls = StringArray::from(vec![doc.url.clone()]);
        let titles = StringArray::from(vec![doc.title.clone()]);
        let contents = StringArray::from(vec![doc.content.clone()]);
        let checksums = StringArray::from(vec![doc.checksum.clone()]);
        let harvested_at = Int64Array::from(vec![doc.harvested_at.timestamp()]);
        let source_ts = Int64Array::from(vec![doc.source_ts.map(|t| t.timestamp())]);
        let chunk_index = Int32Array::from(vec![doc.chunk_index.map(|x| x as i32)]);
        let chunk_total = Int32Array::from(vec![doc.chunk_total.map(|x| x as i32)]);

        let float_values = Float32Array::from(embedding);
        let embedding_col = FixedSizeListArray::try_new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            EMBEDDING_DIM,
            Arc::new(float_values) as ArrayRef,
            None,
        )
        .map_err(|e| OsmozzError::Storage(format!("Build embedding: {}", e)))?;

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(ids) as ArrayRef,
                Arc::new(sources) as ArrayRef,
                Arc::new(urls) as ArrayRef,
                Arc::new(titles) as ArrayRef,
                Arc::new(contents) as ArrayRef,
                Arc::new(checksums) as ArrayRef,
                Arc::new(harvested_at) as ArrayRef,
                Arc::new(source_ts) as ArrayRef,
                Arc::new(chunk_index) as ArrayRef,
                Arc::new(chunk_total) as ArrayRef,
                Arc::new(embedding_col) as ArrayRef,
            ],
        )
        .map_err(|e| OsmozzError::Storage(format!("Build batch: {}", e)))?;

        let reader = RecordBatchIterator::new(
            vec![Ok::<RecordBatch, ArrowError>(batch)].into_iter(),
            schema,
        );
        table
            .add(reader)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Add row: {}", e)))?;

        debug!("Stored doc: {} chunk {:?}/{:?}", doc.id, doc.chunk_index, doc.chunk_total);
        Ok(())
    }

    pub async fn delete_by_source(&self, source: &str) -> Result<()> {
        let table = self.get_table().await?;
        table
            .delete(&format!("source = '{}'", source))
            .await
            .map_err(|e| OsmozzError::Storage(format!("Delete by source: {}", e)))?;
        Ok(())
    }

    /// Return the most recent docs for a given source, sorted by source_ts DESC.
    pub async fn recent_by_source(&self, source: &str, limit: usize) -> Result<Vec<osmozzz_core::SearchResult>> {
        let table = self.get_table().await?;
        let batches = table
            .query()
            .only_if(format!("source = '{}'", source))
            .limit(100_000) // fetch all, sort in memory
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Recent query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Recent collect: {}", e)))?;

        let mut results: Vec<(i64, osmozzz_core::SearchResult)> = Vec::new();
        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }
            macro_rules! str_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                };
            }
            macro_rules! i32_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                };
            }
            let id_col      = str_col!("id");
            let source_col  = str_col!("source");
            let url_col     = str_col!("url");
            let title_col   = str_col!("title");
            let content_col = str_col!("content");
            let chunk_idx   = i32_col!("chunk_index");
            let chunk_tot   = i32_col!("chunk_total");
            let source_ts_col = batch.column_by_name("source_ts")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at")
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                // Use source_ts (email date) for sorting; fall back to harvested_at only if NULL
                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);

                let title = title_col.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i).to_string()) }
                });
                let content = content_col.map(|c| {
                    let s = c.value(i);
                    if s.len() > 300 {
                        let mut b = 300;
                        while b > 0 && !s.is_char_boundary(b) { b -= 1; }
                        format!("{}…", &s[..b])
                    } else { s.to_string() }
                }).unwrap_or_default();
                let chunk_index = chunk_idx.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i) as u32) });
                let chunk_total = chunk_tot.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i) as u32) });

                results.push((ts, osmozzz_core::SearchResult {
                    id: id_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    score: 1.0,
                    source: source_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    url: url_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    title,
                    content,
                    chunk_index,
                    chunk_total,
                }));
            }
        }
        // Sort by date descending
        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, r)| r).collect())
    }

    pub async fn exists(&self, checksum: &str) -> Result<bool> {
        let table = self.get_table().await?;
        let results = table
            .query()
            .only_if(format!("checksum = '{}'", checksum))
            .limit(1)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Exists query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Exists collect: {}", e)))?;
        Ok(results.iter().any(|b| b.num_rows() > 0))
    }

    pub async fn search(&self, query_embedding: Vec<f32>, limit: usize) -> Result<Vec<SearchResult>> {
        let table = self.get_table().await?;

        let batches = table
            .query()
            .nearest_to(query_embedding)
            .map_err(|e| OsmozzError::Storage(format!("Nearest-to: {}", e)))?
            .limit(limit)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Search execute: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Search collect: {}", e)))?;

        let mut results = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            macro_rules! str_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                };
            }
            macro_rules! i32_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                };
            }

            let id_col      = str_col!("id");
            let source_col  = str_col!("source");
            let url_col     = str_col!("url");
            let title_col   = str_col!("title");
            let content_col = str_col!("content");
            let chunk_idx   = i32_col!("chunk_index");
            let chunk_tot   = i32_col!("chunk_total");
            let score_col   = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for i in 0..nrows {
                let dist = score_col.map(|c| c.value(i)).unwrap_or(1.0);
                let score = (1.0 - dist / 2.0).clamp(0.0, 1.0);

                let title = title_col.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i).to_string()) }
                });

                let content = content_col.map(|c| {
                    let s = c.value(i);
                    if s.len() > 300 {
                        let mut b = 300;
                        while b > 0 && !s.is_char_boundary(b) { b -= 1; }
                        format!("{}…", &s[..b])
                    } else {
                        s.to_string()
                    }
                }).unwrap_or_default();

                let chunk_index = chunk_idx.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i) as u32) }
                });
                let chunk_total = chunk_tot.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i) as u32) }
                });

                results.push(SearchResult {
                    id: id_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    score,
                    source: source_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    url: url_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    title,
                    content,
                    chunk_index,
                    chunk_total,
                });
            }
        }
        Ok(results)
    }

    pub async fn count(&self) -> Result<usize> {
        let table = self.get_table().await?;
        table.count_rows(None)
            .await
            .map_err(|e| OsmozzError::Storage(format!("Count: {}", e)))
    }

    pub async fn count_source(&self, source: &str) -> Result<usize> {
        let table = self.get_table().await?;
        let results = table
            .query()
            .only_if(format!("source = '{}'", source))
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Count source query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Count source collect: {}", e)))?;
        Ok(results.iter().map(|b| b.num_rows()).sum())
    }

    /// Search with an optional source filter (e.g. "email", "chrome").
    pub async fn search_filtered(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
        source_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let table = self.get_table().await?;

        let base = table.query().nearest_to(query_embedding)
            .map_err(|e| OsmozzError::Storage(format!("Nearest-to: {}", e)))?
            .limit(limit);

        let query = if let Some(src) = source_filter {
            base.only_if(format!("source = '{}'", src))
        } else {
            base
        };

        let batches = query
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Search execute: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Search collect: {}", e)))?;

        let mut results = Vec::new();
        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            macro_rules! str_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                };
            }
            macro_rules! i32_col {
                ($name:expr) => {
                    batch.column_by_name($name)
                        .and_then(|c| c.as_any().downcast_ref::<Int32Array>())
                };
            }

            let id_col      = str_col!("id");
            let source_col  = str_col!("source");
            let url_col     = str_col!("url");
            let title_col   = str_col!("title");
            let content_col = str_col!("content");
            let chunk_idx   = i32_col!("chunk_index");
            let chunk_tot   = i32_col!("chunk_total");
            let score_col   = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for i in 0..nrows {
                let dist = score_col.map(|c| c.value(i)).unwrap_or(1.0);
                let score = (1.0 - dist / 2.0).clamp(0.0, 1.0);
                let title = title_col.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i).to_string()) }
                });
                let content = content_col.map(|c| {
                    let s = c.value(i);
                    if s.len() > 300 {
                        let mut b = 300;
                        while b > 0 && !s.is_char_boundary(b) { b -= 1; }
                        format!("{}…", &s[..b])
                    } else {
                        s.to_string()
                    }
                }).unwrap_or_default();
                let chunk_index = chunk_idx.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i) as u32) }
                });
                let chunk_total = chunk_tot.and_then(|c| {
                    if c.is_null(i) { None } else { Some(c.value(i) as u32) }
                });
                results.push(SearchResult {
                    id: id_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    score,
                    source: source_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    url: url_col.map(|c| c.value(i).to_string()).unwrap_or_default(),
                    title,
                    content,
                    chunk_index,
                    chunk_total,
                });
            }
        }
        Ok(results)
    }

    /// Keyword search across ALL email content (from + subject + body).
    /// Same philosophy as filesystem find_file: no ONNX, pure string match.
    /// Finds ANY email containing the keyword regardless of age.
    pub async fn search_emails_by_keyword(&self, keyword: &str, limit: usize) -> Result<Vec<(Option<String>, String, String)>> {
        let table = self.get_table().await?;
        let kw = keyword.to_lowercase();

        let batches = table
            .query()
            .only_if("source = 'email'")
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Keyword query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Keyword collect: {}", e)))?;

        let mut results: Vec<(i64, Option<String>, String, String)> = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            let content_col   = batch.column_by_name("content").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col     = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let url_col       = batch.column_by_name("url").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_ts_col = batch.column_by_name("source_ts").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                let content = match content_col { Some(c) => c.value(i), None => continue };
                let title = title_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i).to_string()) });

                // Match anywhere: full content OR title
                let found = content.to_lowercase().contains(&kw)
                    || title.as_deref().map(|t| t.to_lowercase().contains(&kw)).unwrap_or(false);
                if !found { continue; }

                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);
                let url = url_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                results.push((ts, title, url, content.to_string()));
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, title, url, content)| (title, url, content)).collect())
    }

    /// Get emails matching a sender pattern, sorted by date DESC, full content.
    pub async fn get_emails_by_sender(&self, pattern: &str, limit: usize) -> Result<Vec<(Option<String>, String, String)>> {
        let table = self.get_table().await?;
        let pattern_lower = pattern.to_lowercase();

        let batches = table
            .query()
            .only_if("source = 'email'")
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Sender query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Sender collect: {}", e)))?;

        let mut results: Vec<(i64, Option<String>, String, String)> = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            let content_col = batch.column_by_name("content").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col   = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let url_col     = batch.column_by_name("url").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_ts_col = batch.column_by_name("source_ts").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                let content = match content_col { Some(c) => c.value(i), None => continue };
                let title = title_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i).to_string()) });

                // Match anywhere in the full content or title
                let content_lower = content.to_lowercase();
                let title_lower = title.as_deref().unwrap_or("").to_lowercase();
                if !content_lower.contains(&pattern_lower)
                    && !title_lower.contains(&pattern_lower) {
                    continue;
                }

                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);
                let url = url_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                results.push((ts, title, url, content.to_string()));
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, title, url, content)| (title, url, content)).collect())
    }

    /// Get recent emails with full content (no truncation), sorted by date DESC.
    pub async fn recent_emails_full(&self, limit: usize) -> Result<Vec<(Option<String>, String, String)>> {
        let table = self.get_table().await?;

        let batches = table
            .query()
            .only_if("source = 'email'")
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Recent full query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Recent full collect: {}", e)))?;

        let mut results: Vec<(i64, Option<String>, String, String)> = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            let content_col   = batch.column_by_name("content").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col     = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let url_col       = batch.column_by_name("url").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_ts_col = batch.column_by_name("source_ts").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);
                let title   = title_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i).to_string()) });
                let url     = url_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let content = content_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                results.push((ts, title, url, content));
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, title, url, content)| (title, url, content)).collect())
    }

    /// Get emails matching sender pattern AND within a timestamp range.
    pub async fn get_emails_by_sender_and_date(&self, pattern: &str, from_ts: i64, to_ts: i64, limit: usize) -> Result<Vec<(Option<String>, String, String)>> {
        let table = self.get_table().await?;
        let pattern_lower = pattern.to_lowercase();

        let batches = table
            .query()
            .only_if("source = 'email'")
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("SenderDate query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("SenderDate collect: {}", e)))?;

        let mut results: Vec<(i64, Option<String>, String, String)> = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            let content_col   = batch.column_by_name("content").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col     = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let url_col       = batch.column_by_name("url").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_ts_col = batch.column_by_name("source_ts").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);

                if ts < from_ts || ts > to_ts { continue; }

                let content = match content_col { Some(c) => c.value(i), None => continue };
                let title = title_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i).to_string()) });

                // Match anywhere in the full content or title
                let content_lower = content.to_lowercase();
                let title_lower = title.as_deref().unwrap_or("").to_lowercase();
                if !content_lower.contains(&pattern_lower)
                    && !title_lower.contains(&pattern_lower) {
                    continue;
                }

                let url = url_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                results.push((ts, title, url, content.to_string()));
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, title, url, content)| (title, url, content)).collect())
    }

    /// Get emails within a timestamp range, sorted by date DESC, full content.
    pub async fn get_emails_by_date(&self, from_ts: i64, to_ts: i64, limit: usize) -> Result<Vec<(Option<String>, String, String)>> {
        let table = self.get_table().await?;

        let batches = table
            .query()
            .only_if("source = 'email'")
            .limit(100_000)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Date query: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Date collect: {}", e)))?;

        let mut results: Vec<(i64, Option<String>, String, String)> = Vec::new();

        for batch in &batches {
            let nrows = batch.num_rows();
            if nrows == 0 { continue; }

            let content_col   = batch.column_by_name("content").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let title_col     = batch.column_by_name("title").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let url_col       = batch.column_by_name("url").and_then(|c| c.as_any().downcast_ref::<StringArray>());
            let source_ts_col = batch.column_by_name("source_ts").and_then(|c| c.as_any().downcast_ref::<Int64Array>());
            let harvested_col = batch.column_by_name("harvested_at").and_then(|c| c.as_any().downcast_ref::<Int64Array>());

            for i in 0..nrows {
                let ts = source_ts_col
                    .and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) })
                    .or_else(|| harvested_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i)) }))
                    .unwrap_or(0);

                if ts < from_ts || ts > to_ts { continue; }

                let title   = title_col.and_then(|c| if c.is_null(i) { None } else { Some(c.value(i).to_string()) });
                let url     = url_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                let content = content_col.map(|c| c.value(i).to_string()).unwrap_or_default();
                results.push((ts, title, url, content));
            }
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        Ok(results.into_iter().map(|(_, title, url, content)| (title, url, content)).collect())
    }

    /// Fetch the full content of a document by its URL (no truncation).
    pub async fn get_full_content_by_url(&self, url: &str) -> Result<Option<(Option<String>, String)>> {
        let table = self.get_table().await?;
        let safe_url = url.replace('\'', "''");
        let batches = table
            .query()
            .only_if(format!("url = '{}'", safe_url))
            .limit(1)
            .execute()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Get by url: {}", e)))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|e| OsmozzError::Storage(format!("Get by url collect: {}", e)))?;

        for batch in &batches {
            if batch.num_rows() == 0 { continue; }
            let title = batch.column_by_name("title")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .and_then(|c| if c.is_null(0) { None } else { Some(c.value(0).to_string()) });
            let content = batch.column_by_name("content")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .map(|c| c.value(0).to_string())
                .unwrap_or_default();
            return Ok(Some((title, content)));
        }
        Ok(None)
    }

    /// Merge all fragment files into one and prune old versions.
    /// Run this after bulk indexing to restore fast vector search.
    pub async fn compact(&self) -> Result<()> {
        let table = self.get_table().await?;
        table
            .optimize(OptimizeAction::All)
            .await
            .map_err(|e| OsmozzError::Storage(format!("Compact: {}", e)))?;
        Ok(())
    }
}
