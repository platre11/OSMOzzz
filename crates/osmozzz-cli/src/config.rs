use std::path::PathBuf;

use anyhow::{Context, Result};

pub struct Config {
    pub data_dir: PathBuf,
    pub db_path: PathBuf,
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub socket_path: PathBuf,
    #[allow(dead_code)]
    pub chrome_history_path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self> {
        let home = dirs_next::home_dir().context("Cannot find home directory")?;
        let data_dir = home.join(".osmozzz");
        let db_path = data_dir.join("vault");

        // Look for model in <workspace>/models/ relative to executable, then home
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        // Try workspace root (3 levels up from target/debug/osmozzz)
        let workspace_models = exe_dir
            .ancestors()
            .find(|p| p.join("models").exists())
            .map(|p| p.join("models"))
            .unwrap_or_else(|| PathBuf::from("models"));

        let model_path = workspace_models.join("all-MiniLM-L6-v2.onnx");
        let tokenizer_path = workspace_models.join("tokenizer.json");

        let socket_path = data_dir.join("osmozzz.sock");

        let chrome_history_path = home
            .join("Library/Application Support/Google/Chrome/Default/History");

        Ok(Self {
            data_dir,
            db_path,
            model_path,
            tokenizer_path,
            socket_path,
            chrome_history_path,
        })
    }
}
