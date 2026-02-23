use anyhow::{Context, Result};
use osmozzz_embedder::Vault;

use crate::cli::SearchArgs;
use crate::config::Config;

pub async fn run(args: SearchArgs, cfg: Config) -> Result<()> {
    let vault = Vault::open(
        &cfg.model_path,
        &cfg.tokenizer_path,
        cfg.db_path.to_str().unwrap_or(".osmozzz/vault"),
    )
    .await
    .context("Failed to initialize vault")?;

    let results = if args.source.is_none() {
        // Blended: global top-N + guaranteed email results
        let (mut global, emails) = tokio::try_join!(
            vault.search_filtered(&args.query, args.limit, None),
            vault.search_filtered(&args.query, 2, Some("email"))
        ).context("Search failed")?;
        let seen: std::collections::HashSet<String> =
            global.iter().map(|r| r.id.clone()).collect();
        for r in emails {
            if !seen.contains(&r.id) {
                global.push(r);
            }
        }
        global.sort_by(|a, b| b.score.partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal));
        global
    } else {
        vault.search_filtered(&args.query, args.limit, args.source.as_deref())
            .await
            .context("Search failed")?
    };

    if results.is_empty() {
        println!("No results found for: {}", args.query);
        return Ok(());
    }

    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        _ => {
            println!("\nResults for: \"{}\"\n", args.query);
            println!("{}", "─".repeat(60));

            for (i, r) in results.iter().enumerate() {
                // Ligne source + chunk
                let chunk_info = match (r.chunk_index, r.chunk_total) {
                    (Some(idx), Some(tot)) if tot > 1 => {
                        format!(" [chunk {}/{}]", idx + 1, tot)
                    }
                    _ => String::new(),
                };

                println!(
                    "{}. [{}]{} Score: {:.3}",
                    i + 1,
                    r.source.to_uppercase(),
                    chunk_info,
                    r.score
                );

                if let Some(title) = &r.title {
                    println!("   Title:  {}", title);
                }
                println!("   URL:    {}", r.url);
                println!("   {}", r.content.replace('\n', " "));
                println!("{}", "─".repeat(60));
            }
        }
    }

    Ok(())
}
