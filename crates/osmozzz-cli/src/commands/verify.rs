use anyhow::Result;
use crate::proof;

pub fn run(sig: &str, source: &str, url: &str, content: &str, ts: i64) -> Result<()> {
    let key = proof::load_or_create_key();

    let snippet = &content[..content.len().min(80)];

    if proof::verify_sig(sig, &key, source, url, content, ts) {
        println!("✅ AUTHENTIQUE — Proof of Context vérifié");
        println!("{}", "─".repeat(50));
        println!("Source  : {}", source);
        println!("URL     : {}", url);
        println!("Extrait : {}…", snippet);
        println!("ts      : {}", ts);
        println!("{}", "─".repeat(50));
        println!("Ce snippet provient bien de ta DB locale OSMOzzz.");
        println!("Il n'a pas été modifié depuis sa signature.");
    } else {
        println!("❌ INVALIDE — Proof of Context échoué");
        println!("{}", "─".repeat(50));
        println!("La signature ne correspond pas.");
        println!("Causes possibles :");
        println!("  - Le contenu a été modifié");
        println!("  - La signature vient d'un autre Mac");
        println!("  - Le timestamp est incorrect");
    }

    Ok(())
}
