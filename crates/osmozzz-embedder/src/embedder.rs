use std::path::Path;
use std::sync::Arc;

use osmozzz_core::{OsmozzError, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Tensor as OrtTensor;
use tokenizers::Tokenizer;
use tracing::{debug, info};

/// Local ONNX embedding model (all-MiniLM-L6-v2).
/// Produces 384-dimensional embeddings with no network calls.
pub struct OnnxEmbedder {
    session: Arc<std::sync::Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
}

impl OnnxEmbedder {
    pub fn load(model_path: &Path, tokenizer_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            return Err(OsmozzError::ModelNotFound(model_path.display().to_string()));
        }
        if !tokenizer_path.exists() {
            return Err(OsmozzError::ModelNotFound(tokenizer_path.display().to_string()));
        }

        info!("Loading ONNX model from: {}", model_path.display());
        let session = Session::builder()
            .map_err(|e| OsmozzError::Embedder(format!("ORT session builder: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| OsmozzError::Embedder(format!("ORT optimization: {}", e)))?
            .with_intra_threads(1)
            .map_err(|e| OsmozzError::Embedder(format!("ORT threads: {}", e)))?
            .commit_from_file(model_path)
            .map_err(|e| OsmozzError::Embedder(format!("ORT load model: {}", e)))?;

        info!("Loading tokenizer from: {}", tokenizer_path.display());
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| OsmozzError::Embedder(format!("Tokenizer load: {}", e)))?;

        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
        })
    }

    /// Embed text → 384-dim L2-normalized vector (local inference, no network).
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = &text[..text.len().min(512 * 4)];

        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| OsmozzError::Embedder(format!("Tokenize: {}", e)))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
        let type_ids: Vec<i64> = encoding.get_type_ids().iter().map(|&x| x as i64).collect();
        let seq_len = ids.len();

        // Use (shape_array, vec) tuple — implements OwnedTensorArrayData<T>
        let ids_t = OrtTensor::<i64>::from_array(([1usize, seq_len], ids))
            .map_err(|e| OsmozzError::Embedder(format!("ids tensor: {}", e)))?;
        let mask_t = OrtTensor::<i64>::from_array(([1usize, seq_len], mask))
            .map_err(|e| OsmozzError::Embedder(format!("mask tensor: {}", e)))?;
        let type_ids_t = OrtTensor::<i64>::from_array(([1usize, seq_len], type_ids))
            .map_err(|e| OsmozzError::Embedder(format!("type_ids tensor: {}", e)))?;

        // Lock session for the duration of inference + output extraction
        let mut session_guard = self.session.lock().unwrap();
        let outputs = session_guard
            .run(ort::inputs![
                "input_ids"      => ids_t,
                "attention_mask" => mask_t,
                "token_type_ids" => type_ids_t
            ])
            .map_err(|e| OsmozzError::Embedder(format!("ORT inference: {}", e)))?;

        // try_extract_tensor returns (&Shape, &[f32]) in RC.11
        // Shape is &[i64]: [batch=1, seq_len, hidden_size]
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| OsmozzError::Embedder(format!("Extract tensor: {}", e)))?;

        let hidden_size = shape[2] as usize;

        // Mean pooling over token dimension (manual, from flat slice)
        let mut pooled = vec![0f32; hidden_size];
        for token_idx in 0..seq_len {
            let start = token_idx * hidden_size;
            for dim in 0..hidden_size {
                pooled[dim] += data[start + dim];
            }
        }
        let n = seq_len as f32;
        for v in &mut pooled {
            *v /= n;
        }

        // L2 normalize → cosine similarity ready
        let norm: f32 = pooled.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-9 {
            for v in &mut pooled {
                *v /= norm;
            }
        }

        debug!("Embedded {} tokens → {}d vector", seq_len, hidden_size);
        Ok(pooled)
    }
}
