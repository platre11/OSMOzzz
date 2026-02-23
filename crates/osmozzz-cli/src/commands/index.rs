use anyhow::{bail, Context, Result};
use osmozzz_core::{Embedder, Harvester};
use osmozzz_embedder::Vault;
use osmozzz_harvester::{ChromeHarvester, FileHarvester, GmailConfig, GmailHarvester};

use crate::cli::IndexArgs;
use crate::config::Config;

pub async fn run(args: IndexArgs, cfg: Config) -> Result<()> {
    // Ensure data dir exists
    std::fs::create_dir_all(&cfg.data_dir)
        .context("Cannot create ~/.osmozzz data directory")?;

    let vault = Vault::open(&cfg.model_path, &cfg.tokenizer_path, cfg.db_path.to_str().unwrap())
        .await
        .context("Failed to initialize vault. Make sure the ONNX model is downloaded.")?;

    if args.reset {
        // Map CLI source name → stored source type
        let stored_source = match args.source.as_str() {
            "gmail" => "email",
            other => other,
        };
        println!("Suppression des données existantes pour la source '{}'...", stored_source);
        vault.delete_by_source(stored_source)
            .await
            .context("Échec de la suppression")?;
        println!("Données supprimées. Re-indexation en cours...");
    }

    match args.source.as_str() {
        "chrome" => {
            println!("Indexing Chrome browsing history...");
            let harvester = ChromeHarvester::new();
            let documents = harvester.harvest().await.context("Chrome harvest failed")?;

            if documents.is_empty() {
                println!("No new documents to index.");
                return Ok(());
            }

            println!("Found {} new URLs to embed and index...", documents.len());
            let mut indexed = 0;
            let mut skipped = 0;

            for (i, doc) in documents.iter().enumerate() {
                if i % 50 == 0 && i > 0 {
                    println!("  Progress: {}/{}", i, documents.len());
                }
                match vault.exists(&doc.checksum).await {
                    Ok(true) => {
                        skipped += 1;
                        continue;
                    }
                    _ => {}
                }
                match vault.upsert(doc).await {
                    Ok(_) => indexed += 1,
                    Err(e) => eprintln!("  Warning: Failed to index {}: {}", doc.url, e),
                }
            }

            println!(
                "\nDone! Indexed: {}, Skipped (already indexed): {}",
                indexed, skipped
            );
        }

        "files" => {
            let path = args
                .path
                .context("--path is required when --source=files")?;

            // Expand tilde
            let path = shellexpand::tilde(&path).to_string();

            println!("Indexing files in: {}", path);
            let harvester = FileHarvester::new(&path);
            let documents = harvester.harvest().await.context("File harvest failed")?;

            if documents.is_empty() {
                println!("No new documents found (supported: .md, .txt).");
                return Ok(());
            }

            println!("Found {} new files to embed and index...", documents.len());
            let mut indexed = 0;
            let mut skipped = 0;

            for doc in &documents {
                match vault.exists(&doc.checksum).await {
                    Ok(true) => {
                        skipped += 1;
                        continue;
                    }
                    _ => {}
                }
                match vault.upsert(doc).await {
                    Ok(_) => {
                        indexed += 1;
                        println!("  + {}", doc.url);
                    }
                    Err(e) => eprintln!("  Warning: Failed to index {}: {}", doc.url, e),
                }
            }

            println!(
                "\nDone! Indexed: {}, Skipped (already indexed): {}",
                indexed, skipped
            );
        }

        "gmail" => {
            let config = GmailConfig::load().ok_or_else(|| anyhow::anyhow!(
                "Gmail non configuré.\n\
                 Créez le fichier ~/.osmozzz/gmail.toml avec :\n\
                 \n\
                   username = \"votre@gmail.com\"\n\
                   password = \"xxxx xxxx xxxx xxxx\"\n\
                 \n\
                 Le mot de passe est un mot de passe d'APPLICATION Google\n\
                 (pas votre mot de passe principal).\n\
                 Générez-en un sur : myaccount.google.com/apppasswords\n\
                 \n\
                 Ou via variables d'environnement :\n\
                   OSMOZZZ_GMAIL_USER=votre@gmail.com\n\
                   OSMOZZZ_GMAIL_PASSWORD=xxxx xxxx xxxx xxxx"
            ))?;

            println!("Connexion à Gmail ({})", config.username);
            let harvester = GmailHarvester::new(config);
            let documents = harvester.harvest().await.context("Gmail harvest échoué")?;

            if documents.is_empty() {
                println!("Aucun nouvel email à indexer.");
                return Ok(());
            }

            println!("Récupéré {} emails, indexation en cours...", documents.len());
            let mut indexed = 0;
            let mut skipped = 0;

            for (i, doc) in documents.iter().enumerate() {
                if i % 50 == 0 && i > 0 {
                    println!("  Progression : {}/{}", i, documents.len());
                }
                match vault.exists(&doc.checksum).await {
                    Ok(true) => { skipped += 1; continue; }
                    _ => {}
                }
                match vault.upsert(doc).await {
                    Ok(_) => indexed += 1,
                    Err(e) => eprintln!("  Avertissement : échec indexation email : {}", e),
                }
            }

            println!("\nTerminé ! Indexés : {}, Ignorés (déjà indexés) : {}", indexed, skipped);
        }

        other => {
            bail!(
                "Unknown source '{}'. Supported sources: chrome, files, gmail",
                other
            );
        }
    }

    Ok(())
}
