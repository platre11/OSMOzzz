use regex::Regex;
use super::FilterRule;

pub struct IbanRule {
    re: Regex,
}

impl IbanRule {
    pub fn new() -> Self {
        Self {
            // Format IBAN international : 2 lettres pays + 2 chiffres + jusqu'à 30 alphanum
            // Supporte les espaces entre groupes (FR76 3000 6000...)
            re: Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]{4}){2,7}(?:[ ]?[A-Z0-9]{1,3})?\b").unwrap(),
        }
    }
}

impl FilterRule for IbanRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[IBAN masqué]").to_string()
    }
}
