use regex::Regex;
use super::FilterRule;

pub struct PhoneRule {
    re: Regex,
}

impl PhoneRule {
    pub fn new() -> Self {
        Self {
            re: Regex::new(concat!(
                r"(?x)\b(",
                // France : 06 12 34 56 78 | +33 6 12 34 56 78 | 0033 6 ...
                r"(?:(?:\+33|0033)\s?[1-9](?:[\s.\-]?\d{2}){4}",
                r"|0[1-9](?:[\s.\-]?\d{2}){4})",
                r"|",
                // International générique : +XX ou +XXX suivi de 6–12 chiffres
                // Couvre BE +32, CH +41, UK +44, US +1, DE +49, ES +34, IT +39, MA +212…
                r"\+\d{1,3}[\s.\-]?\(?\d{1,4}\)?(?:[\s.\-]?\d{2,4}){2,4}",
                r")\b",
            )).unwrap(),
        }
    }
}

impl FilterRule for PhoneRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[téléphone masqué]").to_string()
    }
}
