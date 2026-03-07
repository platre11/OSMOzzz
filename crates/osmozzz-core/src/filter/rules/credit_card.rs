use regex::Regex;
use super::FilterRule;

pub struct CreditCardRule {
    re: Regex,
}

impl CreditCardRule {
    pub fn new() -> Self {
        Self {
            // Détecte les suites de 16 chiffres séparés ou non par espaces/tirets
            re: Regex::new(r"\b(?:\d{4}[ \-]?){3}\d{4}\b").unwrap(),
        }
    }
}

impl FilterRule for CreditCardRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, |caps: &regex::Captures| {
            let raw = caps[0].replace([' ', '-'], "");
            if luhn_check(&raw) {
                "[CB masquée]".to_string()
            } else {
                caps[0].to_string()
            }
        }).to_string()
    }
}

/// Algorithme de Luhn — valide qu'un numéro est bien une CB, pas une date ou un code
fn luhn_check(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }
    let sum: u32 = digits.iter().rev().enumerate().map(|(i, &d)| {
        if i % 2 == 1 {
            let v = d * 2;
            if v > 9 { v - 9 } else { v }
        } else {
            d
        }
    }).sum();
    sum % 10 == 0
}
