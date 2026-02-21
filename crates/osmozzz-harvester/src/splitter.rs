/// Découpe un texte en chunks avec chevauchement (overlap).
///
/// Paramètres recommandés pour all-MiniLM-L6-v2 (limite 512 tokens) :
/// - max_chars  : 2000 (~500 tokens, marge de sécurité)
/// - overlap    : 200  (préserve le contexte entre chunks)
pub fn split_text(text: &str, max_chars: usize, overlap_chars: usize) -> Vec<String> {
    let text = text.trim();

    if text.is_empty() {
        return vec![];
    }

    // Texte court → un seul chunk
    if text.len() <= max_chars {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = (start + max_chars).min(text.len());

        let split_at = if end >= text.len() {
            end
        } else {
            find_split_point(text, start, end)
        };

        let chunk = text[start..split_at].trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }

        if split_at <= start {
            // Sécurité : avance d'au moins 1 char pour éviter une boucle infinie
            start += 1;
            continue;
        }

        // Recul de overlap_chars pour le prochain chunk
        start = split_at.saturating_sub(overlap_chars);

        // S'assurer qu'on avance toujours
        if start >= split_at {
            start = split_at;
        }
    }

    chunks
}

/// Trouve le meilleur point de coupure dans `text[start..end]`.
/// Priorité : paragraphe > saut de ligne > fin de phrase > espace.
fn find_split_point(text: &str, start: usize, end: usize) -> usize {
    let segment = &text[start..end];

    if let Some(i) = segment.rfind("\n\n") {
        return start + i + 2;
    }
    if let Some(i) = segment.rfind('\n') {
        return start + i + 1;
    }
    if let Some(i) = segment.rfind(". ") {
        return start + i + 2;
    }
    if let Some(i) = segment.rfind(' ') {
        return start + i + 1;
    }

    // Pas de bon point → coupe dur
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_text_single_chunk() {
        let chunks = split_text("Hello world", 2000, 200);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_long_text_multiple_chunks() {
        let long = "word ".repeat(600); // ~3000 chars
        let chunks = split_text(&long, 2000, 200);
        assert!(chunks.len() >= 2);
        // Chaque chunk respecte la limite
        for chunk in &chunks {
            assert!(chunk.len() <= 2000 + 5); // +5 tolérance bord de mot
        }
    }

    #[test]
    fn test_overlap_present() {
        // Crée un texte avec des marqueurs clairs
        let text = format!("{}MARKER{}", "a ".repeat(950), "b ".repeat(950));
        let chunks = split_text(&text, 2000, 200);
        // MARKER doit apparaître dans au moins un chunk
        assert!(chunks.iter().any(|c| c.contains("MARKER")));
    }

    #[test]
    fn test_empty_text() {
        let chunks = split_text("", 2000, 200);
        assert!(chunks.is_empty());
    }
}
