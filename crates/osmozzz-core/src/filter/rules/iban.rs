use regex::Regex;
use super::FilterRule;

pub struct IbanRule {
    re: Regex,
}

impl IbanRule {
    pub fn new() -> Self {
        Self {
            re: Regex::new(concat!(
                // IBAN international : FR76 3000 6000 0112 3456 7890 189
                r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,3})?\b",
                r"|",
                // RIB français brut : 5 chiffres banque + 5 guichet + 11 compte + 2 clé
                // Ex: 30006 00011 12345678901 89  ou  30006000111234567890189
                r"\b\d{5} ?\d{5} ?\d{11} ?\d{2}\b"
            )).expect("IBAN regex invalide"),
        }
    }
}

impl FilterRule for IbanRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[IBAN masqué]").to_string()
    }
}
