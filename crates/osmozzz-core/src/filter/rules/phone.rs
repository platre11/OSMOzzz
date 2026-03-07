use regex::Regex;
use super::FilterRule;

pub struct PhoneRule {
    re: Regex,
}

impl PhoneRule {
    pub fn new() -> Self {
        Self {
            // Numéros français : 06 12 34 56 78 | +33 6 12 34 56 78 | 0033612345678
            re: Regex::new(
                r"\b(?:(?:\+33|0033)\s?[1-9](?:[\s.\-]?\d{2}){4}|0[1-9](?:[\s.\-]?\d{2}){4})\b"
            ).unwrap(),
        }
    }
}

impl FilterRule for PhoneRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[téléphone masqué]").to_string()
    }
}
