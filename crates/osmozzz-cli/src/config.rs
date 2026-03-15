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

        // 1. ~/.osmozzz/models/ (prioritaire — fonctionne après reboot)
        // 2. workspace/models/ (fallback dev — remonte les ancêtres du binaire)
        let osmozzz_models = data_dir.join("models");

        let models_dir = if osmozzz_models.join("all-MiniLM-L6-v2.onnx").exists() {
            osmozzz_models
        } else {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            exe_dir
                .ancestors()
                .find(|p| p.join("models").join("all-MiniLM-L6-v2.onnx").exists())
                .map(|p| p.join("models"))
                .unwrap_or(osmozzz_models)
        };

        let model_path = models_dir.join("all-MiniLM-L6-v2.onnx");
        let tokenizer_path = models_dir.join("tokenizer.json");

        let socket_path = data_dir.join("osmozzz.sock");

        let chrome_history_path = {
            #[cfg(target_os = "macos")]
            { home.join("Library/Application Support/Google/Chrome/Default/History") }
            #[cfg(target_os = "windows")]
            { dirs_next::data_local_dir().unwrap_or_else(|| home.clone())
                .join("Google/Chrome/User Data/Default/History") }
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            { home.join(".config/google-chrome/Default/History") }
        };

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
